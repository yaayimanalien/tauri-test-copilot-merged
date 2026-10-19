#[cfg(windows)]
mod platform {
    use futures::executor::block_on;
    use windows::{
        core::HSTRING,
        Security::Credentials::UI::{
            UserConsentVerificationResult, UserConsentVerifier, UserConsentVerifierAvailability,
        },
        Win32::System::WinRT::{
            IUserConsentVerifierInterop, RoGetActivationFactory, RoInitialize, RoUninitialize,
            RO_INIT_MULTITHREADED,
        },
    };

    struct WinRtApartment;

    impl WinRtApartment {
        fn initialize() -> Result<Self, String> {
            unsafe { RoInitialize(RO_INIT_MULTITHREADED) }
                .map_err(|error| format!("Couldn't initialize Windows Runtime: {error}"))?;
            Ok(Self)
        }
    }

    impl Drop for WinRtApartment {
        fn drop(&mut self) {
            unsafe { RoUninitialize() };
        }
    }

    fn check_available() -> Result<bool, String> {
        let operation = UserConsentVerifier::CheckAvailabilityAsync()
            .map_err(|error| format!("Couldn't check Windows Hello availability: {error}"))?;
        block_on(async {
            operation
                .await
                .map(|availability| availability == UserConsentVerifierAvailability::Available)
                .map_err(|error| format!("Couldn't check Windows Hello availability: {error}"))
        })
    }

    pub async fn available() -> Result<bool, String> {
        tauri::async_runtime::spawn_blocking(|| {
            let _apartment = WinRtApartment::initialize()?;
            check_available()
        })
        .await
        .map_err(|error| format!("Windows Hello availability check failed: {error}"))?
    }

    fn verify_sync(hwnd: isize) -> Result<(), String> {
        let _apartment = WinRtApartment::initialize()?;
        if !check_available()? {
            return Err("Windows Hello isn't available or configured for this account".into());
        }
        let class_name = HSTRING::from("Windows.Security.Credentials.UI.UserConsentVerifier");
        let message = HSTRING::from("Verify your identity to unlock Daylight");
        let operation: windows_future::IAsyncOperation<UserConsentVerificationResult> = {
            let interop: IUserConsentVerifierInterop = unsafe {
                RoGetActivationFactory(&class_name)
                    .map_err(|error| format!("Couldn't initialize Windows Hello: {error}"))?
            };
            unsafe {
                interop
                    .RequestVerificationForWindowAsync::<
                        windows_future::IAsyncOperation<UserConsentVerificationResult>,
                    >(
                        windows::Win32::Foundation::HWND(hwnd as *mut _),
                        &message,
                    )
                    .map_err(|error| {
                        format!("Couldn't start Windows Hello verification: {error}")
                    })?
            }
        };
        let result = block_on(async {
            operation
                .await
                .map_err(|error| format!("Windows Hello verification failed: {error}"))
        })?;
        if result != UserConsentVerificationResult::Verified {
            return Err("Windows Hello verification was not completed".into());
        }
        Ok(())
    }

    pub async fn verify(hwnd: isize) -> Result<(), String> {
        tauri::async_runtime::spawn_blocking(move || verify_sync(hwnd))
            .await
            .map_err(|error| format!("Windows Hello verification failed: {error}"))?
    }
}

#[cfg(windows)]
pub use platform::available;

/// Mobile (Android / iOS) biometrics. The prompt is shown and checked here in
/// Rust, so the webview cannot skip it by calling the unlock command directly.
#[cfg(mobile)]
pub mod mobile {
    use tauri::{AppHandle, Runtime};
    use tauri_plugin_biometric::{AuthOptions, BiometricExt};

    pub fn available<R: Runtime>(app: &AppHandle<R>) -> Result<bool, String> {
        app.biometric()
            .status()
            .map(|status| status.is_available)
            .map_err(|error| format!("Couldn't check biometric availability: {error}"))
    }

    pub async fn verify<R: Runtime>(app: &AppHandle<R>, reason: &str) -> Result<(), String> {
        let app = app.clone();
        let reason = reason.to_string();
        tauri::async_runtime::spawn_blocking(move || {
            app.biometric()
                .authenticate(reason, AuthOptions::default())
                .map_err(|error| format!("Biometric verification failed: {error}"))
        })
        .await
        .map_err(|error| format!("Biometric verification failed: {error}"))?
    }
}

#[cfg(windows)]
pub async fn verify(hwnd: isize) -> Result<(), String> {
    platform::verify(hwnd).await
}

#[cfg(all(test, windows))]
mod tests {
    use super::available;

    #[test]
    fn windows_hello_availability_query_completes() {
        let result = tauri::async_runtime::block_on(available());
        assert!(result.is_ok(), "{result:?}");
    }
}
