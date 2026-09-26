package com.madhyamas.vpn.pairing

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.UUID

/**
 * CredentialStore policy logic over fakes, plus InstallationId semantics:
 * what survives forget, what survives restart, what never decrypts twice.
 */
class CredentialStoreTest {

    private fun store(aead: Aead = FakeAead(), kv: KeyValueStore = InMemoryKeyValueStore()) =
        CredentialStore(kv, aead)

    @Test
    fun storeAndReadRoundTrip() {
        val s = store()
        s.store("mdy_dev_k", deviceId = "d-1", deviceName = "Pixel")
        assertEquals("mdy_dev_k", s.deviceKey())
        assertEquals("d-1", s.deviceId())
        assertEquals("Pixel", s.deviceName())
        assertTrue(s.isEnrolled())
    }

    @Test
    fun unenrolledStoreHasNoKey() {
        val s = store()
        assertNull(s.deviceKey())
        assertNull(s.deviceId())
        assertNull(s.deviceName())
        assertFalse(s.isEnrolled())
    }

    @Test
    fun onlyCiphertextIsPersisted() {
        val kv = InMemoryKeyValueStore()
        CredentialStore(kv, FakeAead()).store("mdy_dev_secretvalue", null, null)
        val persisted = kv.map.values.joinToString("|")
        assertFalse("plaintext key must not be persisted", persisted.contains("mdy_dev_secretvalue"))
    }

    @Test
    fun survivesRestartViaSamePersistedState() {
        val kv = InMemoryKeyValueStore()
        CredentialStore(kv, FakeAead()).store("mdy_dev_k", "d-1", "Pixel")
        // "Restart" = a fresh store instance over the same persisted bytes.
        val reopened = CredentialStore(kv, FakeAead())
        assertEquals("mdy_dev_k", reopened.deviceKey())
        assertEquals("Pixel", reopened.deviceName())
    }

    @Test
    fun tamperedBlobYieldsNullNotThrow() {
        val kv = InMemoryKeyValueStore()
        CredentialStore(kv, FakeAead()).store("mdy_dev_k", null, null)
        val blob = kv.getBytes("device_key_sealed")!!
        blob[20] = (blob[20].toInt() xor 0x41).toByte()
        kv.putBytes("device_key_sealed", blob)
        val s = CredentialStore(kv, FakeAead())
        assertNull("tampered blob must decrypt to null", s.deviceKey())
    }

    @Test
    fun wrongKeyYieldsNullNotThrow() {
        val kv = InMemoryKeyValueStore()
        CredentialStore(kv, FakeAead(keyId = 1)).store("mdy_dev_k", null, null)
        // Keystore key regenerated (e.g. app data wiped, blob restored via
        // backup) — decrypt must fail closed, not throw.
        assertNull(CredentialStore(kv, FakeAead(keyId = 2)).deviceKey())
    }

    @Test
    fun clearRemovesCredentialAndMetadata() {
        val s = store()
        s.store("mdy_dev_k", "d-1", "Pixel")
        s.clear()
        assertNull(s.deviceKey())
        assertNull(s.deviceId())
        assertNull(s.deviceName())
        assertFalse(s.isEnrolled())
    }

    @Test
    fun clearPreservesInstallationId() {
        val kv = InMemoryKeyValueStore()
        val s = CredentialStore(kv, FakeAead())
        val installId = InstallationId.getOrCreate(kv)
        s.store("mdy_dev_k", "d-1", "Pixel")
        s.clear() // "Forget device"
        assertEquals(
            "install UUID must survive forget (stable device identity)",
            installId,
            InstallationId.getOrCreate(kv)
        )
    }

    @Test
    fun reEnrollAfterClearStoresFreshCredential() {
        val s = store()
        s.store("mdy_dev_old", "d-1", "Old")
        s.clear()
        s.store("mdy_dev_new", "d-2", "New")
        assertEquals("mdy_dev_new", s.deviceKey())
        assertEquals("d-2", s.deviceId())
    }
}

class InstallationIdTest {

    @Test
    fun stableAcrossCallsAndInstances() {
        val kv = InMemoryKeyValueStore()
        val first = InstallationId.getOrCreate(kv)
        assertEquals(first, InstallationId.getOrCreate(kv))
        assertEquals(first, InstallationId.getOrCreate(InMemoryKeyValueStore().also {
            it.map.putAll(kv.map)
        }))
    }

    @Test
    fun distinctInstallationsGetDistinctIds() {
        val a = InstallationId.getOrCreate(InMemoryKeyValueStore())
        val b = InstallationId.getOrCreate(InMemoryKeyValueStore())
        org.junit.Assert.assertNotEquals(a, b)
    }

    @Test
    fun isAUuid() {
        val id = InstallationId.getOrCreate(InMemoryKeyValueStore())
        assertNotNull(UUID.fromString(id)) // throws if not a UUID
    }
}
