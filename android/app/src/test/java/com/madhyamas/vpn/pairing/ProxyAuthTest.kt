package com.madhyamas.vpn.pairing

import org.junit.Assert.assertEquals
import org.junit.Test
import java.util.Base64

/** The exact Proxy-Authorization value the companion authors (issue #111). */
class ProxyAuthTest {

    @Test
    fun basicHeaderValueIsBase64OfKeyWithEmptyPassword() {
        val key = "mdy_dev_0123456789abcdef0123456789abcdef"
        val expected = "Basic " + Base64.getEncoder()
            .encodeToString("$key:".toByteArray(Charsets.UTF_8))
        assertEquals(expected, ProxyAuth.basicHeaderValue(key))
    }

    @Test
    fun headerNameMatchesHttpProxyAuthHeader() {
        assertEquals("Proxy-Authorization", ProxyAuth.HEADER_NAME)
    }

    @Test
    fun differentKeysProduceDifferentHeaders() {
        val a = ProxyAuth.basicHeaderValue("mdy_dev_aaaa")
        val b = ProxyAuth.basicHeaderValue("mdy_dev_bbbb")
        org.junit.Assert.assertNotEquals(a, b)
    }
}
