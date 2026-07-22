package com.madhyamas.vpn.config

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.*
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

/**
 * Persistent proxy configuration stored via DataStore.
 */
private val Context.dataStore: DataStore<Preferences> by preferencesDataStore("proxy_config")

data class ProxyConfig(
    val proxyHost: String = "127.0.0.1",
    val proxyPort: Int = 8888,
    val apiHost: String = "127.0.0.1",
    val apiPort: Int = 3001,
    val selectedPackages: Set<String> = emptySet(),
    val excludeSystemApps: Boolean = true
)

class ConfigManager(private val context: Context) {

    private object Keys {
        val PROXY_HOST = stringPreferencesKey("proxy_host")
        val PROXY_PORT = intPreferencesKey("proxy_port")
        val API_HOST = stringPreferencesKey("api_host")
        val API_PORT = intPreferencesKey("api_port")
        val SELECTED_PACKAGES = stringSetPreferencesKey("selected_packages")
        val EXCLUDE_SYSTEM_APPS = booleanPreferencesKey("exclude_system_apps")
    }

    val configFlow: Flow<ProxyConfig> = context.dataStore.data.map { prefs ->
        ProxyConfig(
            proxyHost = prefs[Keys.PROXY_HOST] ?: "127.0.0.1",
            proxyPort = prefs[Keys.PROXY_PORT] ?: 8888,
            apiHost = prefs[Keys.API_HOST] ?: "127.0.0.1",
            apiPort = prefs[Keys.API_PORT] ?: 3001,
            selectedPackages = prefs[Keys.SELECTED_PACKAGES] ?: emptySet(),
            excludeSystemApps = prefs[Keys.EXCLUDE_SYSTEM_APPS] ?: true
        )
    }

    suspend fun updateProxyHost(host: String) {
        context.dataStore.edit { it[Keys.PROXY_HOST] = host }
    }

    suspend fun updateProxyPort(port: Int) {
        context.dataStore.edit { it[Keys.PROXY_PORT] = port }
    }

    suspend fun updateApiHost(host: String) {
        context.dataStore.edit { it[Keys.API_HOST] = host }
    }

    suspend fun updateApiPort(port: Int) {
        context.dataStore.edit { it[Keys.API_PORT] = port }
    }

    suspend fun updateSelectedPackages(packages: Set<String>) {
        context.dataStore.edit { it[Keys.SELECTED_PACKAGES] = packages }
    }

    suspend fun updateExcludeSystemApps(exclude: Boolean) {
        context.dataStore.edit { it[Keys.EXCLUDE_SYSTEM_APPS] = exclude }
    }
}
