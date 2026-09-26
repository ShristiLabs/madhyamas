package com.madhyamas.vpn.pairing

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import kotlin.io.encoding.Base64
import kotlin.io.encoding.ExperimentalEncodingApi

/**
 * Authenticated encryption used to seal the device credential at rest.
 * [encrypt] returns `iv || ciphertext+tag`; [decrypt] throws on tampering
 * or key mismatch.
 */
interface Aead {
    fun encrypt(plaintext: ByteArray): ByteArray
    fun decrypt(sealed: ByteArray): ByteArray
}

/**
 * Minimal synchronous key-value persistence (a null value removes the
 * entry). The VPN service reads the credential synchronously at start, so
 * DataStore's suspend API is deliberately not used here.
 */
interface KeyValueStore {
    fun getBytes(key: String): ByteArray?
    fun putBytes(key: String, value: ByteArray?)
    fun getString(key: String): String?
    fun putString(key: String, value: String?)
}

/**
 * Stores the paired device credential: the `mdy_dev_` key is sealed with
 * an [Aead] (Android Keystore in production) and the sealed blob plus
 * non-secret metadata live in a [KeyValueStore].
 *
 * Storage choice (issue #111, recorded): direct AndroidKeyStore AES/GCM +
 * a preferences file — NOT EncryptedSharedPreferences, which is deprecated
 * (androidx.security.crypto). The credential survives app restarts; wiping
 * app data removes both the preferences file and the Keystore key's
 * usefulness (expected behavior).
 */
class CredentialStore(private val kv: KeyValueStore, private val aead: Aead) {

    fun store(deviceKey: String, deviceId: String?, deviceName: String?) {
        kv.putBytes(KEY_DEVICE_KEY, aead.encrypt(deviceKey.toByteArray(Charsets.UTF_8)))
        kv.putString(KEY_DEVICE_ID, deviceId)
        kv.putString(KEY_DEVICE_NAME, deviceName)
    }

    /** The decrypted device key, or null when unpaired or undecryptable (e.g. Keystore invalidated). */
    fun deviceKey(): String? {
        val sealed = kv.getBytes(KEY_DEVICE_KEY) ?: return null
        return try {
            String(aead.decrypt(sealed), Charsets.UTF_8)
        } catch (e: Exception) {
            null
        }
    }

    fun deviceId(): String? = kv.getString(KEY_DEVICE_ID)

    fun deviceName(): String? = kv.getString(KEY_DEVICE_NAME)

    fun isEnrolled(): Boolean = kv.getBytes(KEY_DEVICE_KEY) != null

    /**
     * Removes the credential and its metadata only. The installation UUID
     * deliberately survives (stable device identity across re-pairings).
     */
    fun clear() {
        kv.putBytes(KEY_DEVICE_KEY, null)
        kv.putString(KEY_DEVICE_ID, null)
        kv.putString(KEY_DEVICE_NAME, null)
    }

    companion object {
        private const val KEY_DEVICE_KEY = "device_key_sealed"
        private const val KEY_DEVICE_ID = "device_id"
        private const val KEY_DEVICE_NAME = "device_name"
    }
}

/**
 * AES/GCM key held inside the Android Keystore (hardware-backed where
 * available). The key never leaves the Keystore; only sealed blobs are
 * persisted. Thin Android-only shim — all policy lives in [CredentialStore].
 */
class KeystoreAead(private val alias: String = DEFAULT_ALIAS) : Aead {

    private val key: SecretKey by lazy { getOrCreateKey() }

    override fun encrypt(plaintext: ByteArray): ByteArray {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, key)
        val iv = cipher.iv
        val ct = cipher.doFinal(plaintext)
        return iv + ct
    }

    override fun decrypt(sealed: ByteArray): ByteArray {
        require(sealed.size > GCM_IV_LEN) { "sealed blob too short" }
        val iv = sealed.copyOfRange(0, GCM_IV_LEN)
        val ct = sealed.copyOfRange(GCM_IV_LEN, sealed.size)
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(GCM_TAG_BITS, iv))
        return cipher.doFinal(ct)
    }

    private fun getOrCreateKey(): SecretKey {
        val ks = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        (ks.getKey(alias, null) as? SecretKey)?.let { return it }
        val generator = KeyGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE
        )
        generator.init(
            KeyGenParameterSpec.Builder(
                alias,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build()
        )
        return generator.generateKey()
    }

    companion object {
        const val DEFAULT_ALIAS = "madhyamas_device_credential"
        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val TRANSFORMATION = "AES/GCM/NoPadding"
        private const val GCM_IV_LEN = 12
        private const val GCM_TAG_BITS = 128
    }
}

/**
 * [KeyValueStore] over a private SharedPreferences file. Byte values are
 * stored as Base64 strings (the blob is ciphertext — the Keystore key is
 * the actual protection).
 */
@OptIn(ExperimentalEncodingApi::class)
class PrefsKeyValueStore(context: Context, name: String = "device_credential") : KeyValueStore {

    private val prefs = context.getSharedPreferences(name, Context.MODE_PRIVATE)

    override fun getBytes(key: String): ByteArray? {
        val text = prefs.getString(key, null) ?: return null
        return try {
            Base64.Default.decode(text)
        } catch (e: Exception) {
            null
        }
    }

    override fun putBytes(key: String, value: ByteArray?) {
        if (value == null) {
            prefs.edit().remove(key).apply()
        } else {
            prefs.edit().putString(key, Base64.Default.encode(value)).apply()
        }
    }

    override fun getString(key: String): String? = prefs.getString(key, null)

    override fun putString(key: String, value: String?) {
        if (value == null) {
            prefs.edit().remove(key).apply()
        } else {
            prefs.edit().putString(key, value).apply()
        }
    }
}

/**
 * Stable per-installation identity (issue #111 scope): generated once at
 * first run and kept in plain storage — it is an identifier, not a secret.
 * It is deliberately NOT sent to the server: the enroll endpoint schema
 * (#106) has no field for it (recorded as an explicit follow-up).
 */
object InstallationId {
    private const val KEY = "install_uuid"

    fun getOrCreate(kv: KeyValueStore): String {
        kv.getString(KEY)?.let { return it }
        val id = java.util.UUID.randomUUID().toString()
        kv.putString(KEY, id)
        return id
    }
}
