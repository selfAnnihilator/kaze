package org.intelligentmusicplayer.kazemedia

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

/** Credential storage backed by the Android Keystore (AES-256 master key). */
class SecretStore(context: Context) {
    private val prefs = EncryptedSharedPreferences.create(
        context,
        FILE_NAME,
        MasterKey.Builder(context).setKeyScheme(MasterKey.KeyScheme.AES256_GCM).build(),
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
    )

    fun set(key: String, value: String) = prefs.edit().putString(key, value).apply()

    fun get(key: String): String? = prefs.getString(key, null)

    fun delete(key: String) = prefs.edit().remove(key).apply()

    private companion object {
        const val FILE_NAME = "kaze_secrets"
    }
}
