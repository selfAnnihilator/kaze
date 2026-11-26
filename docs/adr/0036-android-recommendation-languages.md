# Android song language preferences

Android Music Taste accepts multiple song languages and all offered genres.
Store the language choices as a JSON array in a profile-scoped
`application_settings` key. Save it in the same transaction as the taste
setup revision and starter pool reset. This uses the existing schema. An
omitted language field preserves the stored value for older clients. An empty
array clears it.

With a nonempty selection, personalized song candidates use the selected
languages plus languages established by qualified listening history. An
isolated search or brief skip adds no language. The recommendation coordinator
uses catalog language metadata to seed candidates and refreshes the account's
starter pool when the effective languages change. Existing profiles with no
language selection continue through the general starter path.

Android records a missing taste setup as skipped on successful sign-in to an
existing account, before the session becomes visible to the UI. Listening
history sync runs in the background and can complete after the onboarding
query. An existing saved setup takes precedence over this marker. Sign-up and
the signed-out first run use the existing setup rules.
