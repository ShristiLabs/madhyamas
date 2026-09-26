package com.madhyamas.vpn.pairing

import java.security.MessageDigest

/** In-memory [KeyValueStore] shared by the pairing tests. */
class InMemoryKeyValueStore : KeyValueStore {
    val map = HashMap<String, Any>()

    override fun getBytes(key: String): ByteArray? = map[key] as? ByteArray

    override fun putBytes(key: String, value: ByteArray?) {
        if (value == null) map.remove(key) else map[key] = value
    }

    override fun getString(key: String): String? = map[key] as? String

    override fun putString(key: String, value: String?) {
        if (value == null) map.remove(key) else map[key] = value
    }
}

/**
 * Deterministic, tamper-detecting fake AEAD: sealed = iv || plaintext ||
 * SHA-256(iv || plaintext || keyId). A different [keyId] (wrong key) or a
 * flipped ciphertext byte fails authentication, mirroring AES/GCM.
 */
class FakeAead(private val keyId: Int = 0) : Aead {

    override fun encrypt(plaintext: ByteArray): ByteArray {
        val iv = ByteArray(IV_LEN) { (it * 7 + 1).toByte() }
        return iv + plaintext + tag(iv, plaintext)
    }

    override fun decrypt(sealed: ByteArray): ByteArray {
        require(sealed.size > IV_LEN + TAG_LEN) { "sealed blob too short" }
        val iv = sealed.copyOfRange(0, IV_LEN)
        val ct = sealed.copyOfRange(IV_LEN, sealed.size - TAG_LEN)
        val tag = sealed.copyOfRange(sealed.size - TAG_LEN, sealed.size)
        require(tag.contentEquals(tag(iv, ct))) { "authentication failed" }
        return ct
    }

    private fun tag(iv: ByteArray, plaintext: ByteArray): ByteArray =
        MessageDigest.getInstance("SHA-256")
            .digest(iv + plaintext + byteArrayOf(keyId.toByte()))

    companion object {
        private const val IV_LEN = 12
        private const val TAG_LEN = 32
    }
}
