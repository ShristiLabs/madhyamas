package com.madhyamas.vpn.vpn

import android.content.Context
import android.content.pm.ApplicationInfo
import android.content.pm.PackageManager
import android.graphics.drawable.Drawable

/**
 * Information about an installed app, for the app selector UI.
 */
data class AppInfo(
    val packageName: String,
    val label: String,
    val isSystemApp: Boolean,
    val icon: Drawable?,
    val uid: Int
)

/**
 * Get a list of installed apps, optionally filtering out system apps.
 */
fun getInstalledApps(context: Context, excludeSystem: Boolean = true): List<AppInfo> {
    val pm = context.packageManager
    return pm.getInstalledApplications(PackageManager.GET_META_DATA)
        .filter { appInfo ->
            if (excludeSystem) {
                // Keep only non-system apps (user-installed)
                (appInfo.flags and ApplicationInfo.FLAG_SYSTEM) == 0
            } else {
                true
            }
        }
        .filter { it.packageName != context.packageName } // Exclude self
        .sortedBy { it.loadLabel(pm).toString().lowercase() }
        .map { appInfo ->
            AppInfo(
                packageName = appInfo.packageName,
                label = appInfo.loadLabel(pm).toString(),
                isSystemApp = (appInfo.flags and ApplicationInfo.FLAG_SYSTEM) != 0,
                icon = appInfo.loadIcon(pm),
                uid = appInfo.uid
            )
        }
}
