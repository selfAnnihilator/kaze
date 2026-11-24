package org.intelligentmusicplayer.kazemedia

import android.content.Context
import android.content.SharedPreferences
import android.util.Log
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

/**
 * Credential storage backed by the Android Keystore (AES-256 master key).
 *
 * The encrypted file can outlive its key: data restored from a backup or moved
 * from another phone arrives without the Keystore key, which never leaves the
 * device. Such a file can't be opened, so it is deleted and a new one made;
 * the user signs in again instead of every read and write failing for good.
 */
class SecretStore(private val context: Context) {
    private val prefs: SharedPreferences = try {
        open()
    } catch (error: Exception) {
        Log.w(TAG, "Unreadable secret store (restored without its key); starting a new one", error)
        context.deleteSharedPreferences(FILE_NAME)
        open()
    }

    private fun open(): SharedPreferences = EncryptedSharedPreferences.create(
        context,
        FILE_NAME,
        MasterKey.Builder(context).setKeyScheme(MasterKey.KeyScheme.AES256_GCM).build(),
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
    )

    fun set(key: String, value: String) = prefs.edit().putString(key, value).apply()

    /** A value that can't be decrypted reads as missing, so the app signs out cleanly. */
    fun get(key: String): String? = try {
        prefs.getString(key, null)
    } catch (error: Exception) {
        Log.w(TAG, "Unreadable secret; treating it as missing", error)
        prefs.edit().remove(key).apply()
        null
    }

    fun delete(key: String) = prefs.edit().remove(key).apply()

    private companion object {
        const val FILE_NAME = "kaze_secrets"
        const val TAG = "KazeSecrets"
    }
}
