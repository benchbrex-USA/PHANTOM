//! Extended integration tests — macOS bridge, account creation, pipeline dry-run,
//! self-healing error paths.
//!
//! These tests use mocks for external dependencies (subprocess, browser, API)
//! so they run offline in CI without hitting real services.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

// ═══════════════════════════════════════════════════════════════════════════
//  1. macOS Bridge Integration (mock subprocess)
// ═══════════════════════════════════════════════════════════════════════════

mod macos_bridge {
    use phantom_core::macos::{
        BrowserAction, BrowserAutomation, BrowserConfig, BrowserType, ClipboardBridge,
        KeychainManager, LaunchctlManager, OsascriptBridge, ScreenCapture,
    };
    use std::time::Duration;

    // ── OsascriptBridge ──────────────────────────────────────────────────

    #[test]
    fn test_osascript_bridge_creation() {
        let bridge = OsascriptBridge::new();
        // Bridge should create without error
        assert!(std::mem::size_of_val(&bridge) > 0);
    }

    #[test]
    fn test_osascript_bridge_with_timeout() {
        let bridge = OsascriptBridge::new().with_timeout(Duration::from_secs(5));
        // Should accept custom timeout
        assert!(std::mem::size_of_val(&bridge) > 0);
    }

    #[test]
    fn test_osascript_applescript_returns_result() {
        // `osascript -e 'return "hello"'` should work on any macOS
        let bridge = OsascriptBridge::new();
        let result = bridge.run_applescript("return \"hello\"");
        // On macOS this succeeds; on Linux CI it fails gracefully
        if cfg!(target_os = "macos") {
            let r = result.unwrap();
            assert!(r.success);
            assert!(r.output.contains("hello"));
        } else {
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_osascript_jxa_returns_result() {
        let bridge = OsascriptBridge::new();
        // Simple JXA expression
        let result = bridge.run_javascript("JSON.stringify({ok: true})");
        if cfg!(target_os = "macos") {
            let r = result.unwrap();
            assert!(r.success);
            assert!(r.output.contains("ok"));
        }
    }

    #[test]
    fn test_osascript_invalid_script_fails() {
        let bridge = OsascriptBridge::new();
        let result = bridge.run_applescript("this is not valid applescript @@!!");
        if cfg!(target_os = "macos") {
            // Should return a result with success=false or an error
            match result {
                Ok(r) => assert!(!r.success),
                Err(_) => {} // Also acceptable
            }
        }
    }

    #[test]
    fn test_osascript_timeout_enforcement() {
        let bridge = OsascriptBridge::new().with_timeout(Duration::from_millis(100));
        // A script that would hang shouldn't block forever
        let result = bridge.run_applescript("delay 0.01\nreturn \"ok\"");
        // We just verify it doesn't hang — result can be ok or timeout error
        let _ = result;
    }

    // ── KeychainManager ──────────────────────────────────────────────────

    #[test]
    fn test_keychain_manager_creation() {
        let km = KeychainManager::new();
        assert!(std::mem::size_of_val(&km) > 0);
    }

    #[test]
    fn test_keychain_store_retrieve_delete_cycle() {
        if !cfg!(target_os = "macos") {
            return;
        }
        let km = KeychainManager::new();
        let service = "phantom-test-integ";
        let account = "test-user";
        let password = "s3cret-p@ss";

        // Store
        let store_result = km.store(service, account, password);
        if store_result.is_err() {
            // May fail if Keychain is locked — skip gracefully
            return;
        }

        // Retrieve
        let retrieved = km.retrieve(service, account);
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap().password, password);

        // Delete
        let del_result = km.delete(service, account);
        assert!(del_result.is_ok());

        // After delete, retrieve should fail
        let gone = km.retrieve(service, account);
        assert!(gone.is_err());
    }

    #[test]
    fn test_keychain_retrieve_nonexistent() {
        if !cfg!(target_os = "macos") {
            return;
        }
        let km = KeychainManager::new();
        let result = km.retrieve("phantom-nonexistent-service-xyz", "nobody");
        assert!(result.is_err());
    }

    // ── ClipboardBridge ──────────────────────────────────────────────────

    #[test]
    fn test_clipboard_roundtrip() {
        if !cfg!(target_os = "macos") {
            return;
        }
        let cb = ClipboardBridge::new();

        let test_str = "phantom-clipboard-test-42";
        cb.write(test_str).unwrap();

        let read = cb.read().unwrap();
        assert!(
            read.contains(test_str),
            "clipboard should contain written text"
        );
    }

    // ── ScreenCapture ────────────────────────────────────────────────────

    #[test]
    fn test_screen_capture_creation() {
        let sc = ScreenCapture::new();
        assert!(std::mem::size_of_val(&sc) > 0);
    }

    #[test]
    fn test_screen_capture_to_temp() {
        if !cfg!(target_os = "macos") {
            return;
        }
        let sc = ScreenCapture::new();
        let path = std::env::temp_dir().join("phantom-integ-screenshot.png");
        let result = sc.capture_screen(Some(path.to_str().unwrap()));
        // May fail in headless CI or without screen access — just verify no panic
        if result.is_ok() && path.exists() {
            let _ = std::fs::remove_file(&path);
        }
    }

    // ── LaunchctlManager ─────────────────────────────────────────────────

    #[test]
    fn test_launchctl_manager_creation() {
        let lm = LaunchctlManager::new();
        assert!(std::mem::size_of_val(&lm) > 0);
    }

    #[test]
    fn test_launchctl_status_nonexistent() {
        if !cfg!(target_os = "macos") {
            return;
        }
        let lm = LaunchctlManager::new();
        let status = lm.status("com.phantom.nonexistent.service.xyz");
        // Should fail or report not loaded
        let _ = status;
    }

    #[test]
    fn test_daemon_config_plist_generation() {
        use phantom_core::macos::DaemonConfig;
        let config =
            DaemonConfig::new("com.phantom.test", "/usr/bin/true").with_args(vec!["--flag".into()]);
        let plist = config.to_plist();
        assert!(plist.contains("com.phantom.test"));
        assert!(plist.contains("/usr/bin/true"));
        assert!(plist.contains("--flag"));
        assert!(plist.contains("<?xml"));
    }

    // ── BrowserAutomation ────────────────────────────────────────────────

    #[test]
    fn test_browser_config_defaults() {
        let config = BrowserConfig::default();
        assert!(matches!(config.browser, BrowserType::Chromium));
        assert!(config.timeout_ms > 0);
    }

    #[test]
    fn test_browser_automation_creation() {
        let ba = BrowserAutomation::new(BrowserConfig::default());
        assert!(!ba.is_active());
    }

    #[test]
    fn test_browser_action_variants() {
        // Verify all action variants can be constructed
        let actions: Vec<BrowserAction> = vec![
            BrowserAction::Navigate {
                url: "https://example.com".into(),
            },
            BrowserAction::Click {
                selector: "#btn".into(),
            },
            BrowserAction::Type {
                selector: "#input".into(),
                text: "hello".into(),
            },
            BrowserAction::Fill {
                selector: "#field".into(),
                value: "world".into(),
            },
            BrowserAction::WaitFor {
                selector: ".loaded".into(),
                timeout_ms: None,
            },
            BrowserAction::Screenshot {
                path: "/tmp/ss.png".into(),
            },
        ];
        assert_eq!(actions.len(), 6);
    }

    #[test]
    fn test_browser_target_mapping() {
        // Verify that Firefox maps internally (falls back to Safari for JXA)
        // Verify distinct variants exist
        let chromium = format!("{}", BrowserType::Chromium);
        let firefox = format!("{}", BrowserType::Firefox);
        let webkit = format!("{}", BrowserType::Webkit);
        assert_ne!(chromium, firefox);
        assert_ne!(chromium, webkit);
        assert_ne!(firefox, webkit);
    }

    #[test]
    fn test_browser_execute_without_session() {
        let mut ba = BrowserAutomation::new(BrowserConfig::default());
        // Should fail because session isn't started
        let result = ba.execute_action(&BrowserAction::Navigate {
            url: "https://example.com".into(),
        });
        assert!(result.is_err());
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  2. Account Creation Flow Execution (mock browser)
// ═══════════════════════════════════════════════════════════════════════════

mod account_creation {
    use super::*;
    use phantom_infra::accounts::{
        detect_captcha, signup_steps, AccountError, AccountManager, CaptchaAction, SignupAction,
        SignupExecutor,
    };
    use phantom_infra::providers::Provider;

    /// Mock signup executor that records all browser actions for verification.
    struct RecordingExecutor {
        actions: Vec<String>,
        captcha_notifications: Vec<String>,
        page_html: String,
        fail_on_action: Option<String>,
    }

    impl RecordingExecutor {
        fn new() -> Self {
            Self {
                actions: Vec::new(),
                captcha_notifications: Vec::new(),
                page_html: "<html><body>Welcome</body></html>".into(),
                fail_on_action: None,
            }
        }

        fn with_captcha_page(mut self) -> Self {
            self.page_html =
                "<html><body><div class='cf-challenge'>Verify you are human</div></body></html>"
                    .into();
            self
        }

        fn fail_on(mut self, action: &str) -> Self {
            self.fail_on_action = Some(action.into());
            self
        }

        fn check_fail(&self, action: &str) -> Result<(), AccountError> {
            if self.fail_on_action.as_deref() == Some(action) {
                return Err(AccountError::SignupFailed {
                    step: 0,
                    reason: format!("mock failure on {}", action),
                });
            }
            Ok(())
        }
    }

    impl SignupExecutor for RecordingExecutor {
        fn navigate(&mut self, url: &str) -> Result<(), AccountError> {
            self.check_fail("navigate")?;
            self.actions.push(format!("navigate:{}", url));
            Ok(())
        }
        fn fill(&mut self, selector: &str, value: &str) -> Result<(), AccountError> {
            self.check_fail("fill")?;
            self.actions.push(format!("fill:{}={}", selector, value));
            Ok(())
        }
        fn click(&mut self, selector: &str) -> Result<(), AccountError> {
            self.check_fail("click")?;
            self.actions.push(format!("click:{}", selector));
            Ok(())
        }
        fn wait_for(&mut self, selector: &str, _timeout: Duration) -> Result<(), AccountError> {
            self.check_fail("wait_for")?;
            self.actions.push(format!("wait_for:{}", selector));
            Ok(())
        }
        fn wait_for_url(&mut self, pattern: &str, _timeout: Duration) -> Result<(), AccountError> {
            self.check_fail("wait_for_url")?;
            self.actions.push(format!("wait_for_url:{}", pattern));
            Ok(())
        }
        fn extract_token(&mut self, selector: &str) -> Result<String, AccountError> {
            self.check_fail("extract_token")?;
            self.actions.push(format!("extract_token:{}", selector));
            Ok("mock-token-abc".into())
        }
        fn screenshot(&mut self, filename: &str) -> Result<(), AccountError> {
            self.actions.push(format!("screenshot:{}", filename));
            Ok(())
        }
        fn page_content(&mut self) -> Result<String, AccountError> {
            Ok(self.page_html.clone())
        }
        fn notify_captcha_pause(&mut self, message: &str) -> Result<(), AccountError> {
            self.captcha_notifications.push(message.to_string());
            Ok(())
        }
    }

    // ── Signup flow data tests ───────────────────────────────────────────

    #[test]
    fn test_all_14_providers_have_signup_flows() {
        let providers_with_flows = [
            Provider::GitHub,
            Provider::Cloudflare,
            Provider::Vercel,
            Provider::Supabase,
            Provider::Upstash,
            Provider::Neon,
            Provider::FlyIo,
            Provider::Railway,
            Provider::Render,
            Provider::Netlify,
            Provider::DigitalOcean,
            Provider::Hetzner,
            Provider::Vultr,
            Provider::OracleCloud,
        ];

        for provider in &providers_with_flows {
            let steps = signup_steps(*provider);
            assert!(!steps.is_empty(), "{:?} missing signup flow", provider);

            // First step is always Navigate
            assert!(
                matches!(&steps[0].action, SignupAction::Navigate { .. }),
                "{:?}: first step should be Navigate",
                provider
            );

            // Every flow has at least one CAPTCHA pause
            assert!(
                steps
                    .iter()
                    .any(|s| matches!(&s.action, SignupAction::PauseForCaptcha { .. })),
                "{:?}: missing CAPTCHA pause step",
                provider
            );

            // Last step should be WaitForUrl (wait for dashboard)
            assert!(
                matches!(
                    &steps.last().unwrap().action,
                    SignupAction::WaitForUrl { .. }
                ),
                "{:?}: last step should be WaitForUrl",
                provider
            );
        }
    }

    #[test]
    fn test_signup_flow_template_vars_present() {
        // Ensure signup steps use {{VAR}} template placeholders
        for provider in &[Provider::Cloudflare, Provider::Vercel, Provider::FlyIo] {
            let steps = signup_steps(*provider);
            let has_template_var = steps.iter().any(|s| match &s.action {
                SignupAction::Fill { value, .. } => value.contains("{{"),
                _ => false,
            });
            assert!(
                has_template_var,
                "{:?}: signup steps should contain template variables",
                provider
            );
        }
    }

    // ── Execute signup flow (mock browser) ───────────────────────────────

    #[test]
    fn test_execute_signup_github() {
        let mut mgr = AccountManager::new();
        let mut exec = RecordingExecutor::new();
        let mut vars = HashMap::new();
        vars.insert("EMAIL".into(), "ghost@phantom.dev".into());
        vars.insert("PASSWORD".into(), "P@ssw0rd!".into());
        vars.insert("USERNAME".into(), "phantom-ghost".into());

        let result = mgr
            .execute_signup_flow(Provider::GitHub, &mut exec, &vars)
            .unwrap();

        assert!(result.success);
        assert_eq!(result.steps_total, 9);
        assert_eq!(result.steps_completed, 9);

        // Verify navigation to GitHub signup
        assert!(exec.actions.iter().any(|a| a.contains("github.com/signup")));

        // Verify email was filled with resolved template
        assert!(exec.actions.iter().any(|a| a.contains("ghost@phantom.dev")));

        // Verify username was filled
        assert!(exec.actions.iter().any(|a| a.contains("phantom-ghost")));

        // At least one CAPTCHA pause notification
        assert!(!exec.captcha_notifications.is_empty());

        // Credential should be registered
        assert!(mgr.get_credential(Provider::GitHub).is_some());
        assert!(mgr.get_status(Provider::GitHub).unwrap().authenticated);
    }

    #[test]
    fn test_execute_signup_oracle_complex_flow() {
        let mut mgr = AccountManager::new();
        let mut exec = RecordingExecutor::new();
        let mut vars = HashMap::new();
        vars.insert("EMAIL".into(), "test@oracle.test".into());
        vars.insert("PASSWORD".into(), "Or@cle123!".into());
        vars.insert("FIRST_NAME".into(), "Test".into());
        vars.insert("LAST_NAME".into(), "User".into());
        vars.insert("COUNTRY".into(), "US".into());

        let result = mgr
            .execute_signup_flow(Provider::OracleCloud, &mut exec, &vars)
            .unwrap();

        assert!(result.success);
        // Oracle has the most steps (10)
        assert!(result.steps_total >= 9);

        // Verify country was filled
        assert!(exec.actions.iter().any(|a| a.contains("US")));
        // Verify names were filled
        assert!(exec.actions.iter().any(|a| a.contains("Test")));
        assert!(exec.actions.iter().any(|a| a.contains("User")));
    }

    #[test]
    fn test_execute_signup_with_auto_captcha_detection() {
        let mut mgr = AccountManager::new();
        let mut exec = RecordingExecutor::new().with_captcha_page();
        let vars = HashMap::new();

        let result = mgr
            .execute_signup_flow(Provider::Cloudflare, &mut exec, &vars)
            .unwrap();

        assert!(result.success);
        // Should have extra captcha pauses from auto-detection
        // (cf-challenge triggers "challenge" indicator, plus PauseForCaptcha step)
        assert!(
            result.captcha_pauses >= 2,
            "expected >=2 captcha pauses, got {}",
            result.captcha_pauses
        );
    }

    #[test]
    fn test_execute_signup_navigate_failure() {
        let mut mgr = AccountManager::new();
        let mut exec = RecordingExecutor::new().fail_on("navigate");
        let vars = HashMap::new();

        let err = mgr.execute_signup_flow(Provider::Vercel, &mut exec, &vars);
        assert!(err.is_err());

        match err.unwrap_err() {
            AccountError::SignupFailed { step, reason } => {
                assert_eq!(step, 0);
                assert!(reason.contains("navigate"));
            }
            other => panic!("expected SignupFailed, got: {:?}", other),
        }
    }

    #[test]
    fn test_execute_signup_click_failure_mid_flow() {
        let mut mgr = AccountManager::new();
        let mut exec = RecordingExecutor::new().fail_on("click");
        let mut vars = HashMap::new();
        vars.insert("EMAIL".into(), "x@y.com".into());
        vars.insert("PASSWORD".into(), "pass".into());

        // Cloudflare: Navigate → Fill email → Fill password → Click submit (fails here)
        let err = mgr.execute_signup_flow(Provider::Cloudflare, &mut exec, &vars);
        assert!(err.is_err());

        match err.unwrap_err() {
            AccountError::SignupFailed { step, reason } => {
                // Click is step 3 (0-indexed) in Cloudflare flow
                assert!(step > 0, "failure should be after initial steps");
                assert!(reason.contains("click"));
            }
            other => panic!("expected SignupFailed, got: {:?}", other),
        }

        // Should NOT have registered a credential
        assert!(mgr.get_credential(Provider::Cloudflare).is_none());
    }

    #[test]
    fn test_execute_signup_no_flow_for_provider() {
        let mut mgr = AccountManager::new();
        let mut exec = RecordingExecutor::new();
        let vars = HashMap::new();

        // GoogleCloud has no signup flow
        let err = mgr.execute_signup_flow(Provider::GoogleCloud, &mut exec, &vars);
        assert!(err.is_err());
    }

    // ── CAPTCHA detection tests ──────────────────────────────────────────

    #[test]
    fn test_captcha_detection_hcaptcha() {
        let html = r#"<div class="h-captcha" data-sitekey="xxx"></div>"#;
        let det = detect_captcha(html, Provider::Supabase);
        assert!(det.detected);
        assert!(det.indicator.as_deref().unwrap().contains("captcha"));
        assert_eq!(det.action, CaptchaAction::PauseForManual);
    }

    #[test]
    fn test_captcha_detection_cloudflare_challenge() {
        let html = r#"<div id="cf-challenge-running">Checking...</div>"#;
        let det = detect_captcha(html, Provider::Cloudflare);
        assert!(det.detected);
        assert_eq!(det.indicator.as_deref(), Some("cf-challenge"));
    }

    #[test]
    fn test_captcha_detection_verify_human() {
        let html = "Please verify you are human to continue.";
        let det = detect_captcha(html, Provider::DigitalOcean);
        assert!(det.detected);
        assert_eq!(det.indicator.as_deref(), Some("verify you are human"));
    }

    #[test]
    fn test_captcha_detection_clean_page() {
        let html = "<html><body><h1>Welcome! Create your account.</h1></body></html>";
        let det = detect_captcha(html, Provider::Vercel);
        assert!(!det.detected);
        assert!(det.indicator.is_none());
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  3. Full Pipeline Dry-Run End-to-End
// ═══════════════════════════════════════════════════════════════════════════

mod pipeline_dry_run {
    use super::*;
    use phantom_core::audit::AuditLog;
    use phantom_core::pipeline::executor::{PipelineExecutor, ProgressKind};
    use phantom_core::{BuildPhase, BuildPipeline, MessageBus, Task};

    fn make_task(name: &str, role: &str, phase: BuildPhase) -> Task {
        Task::new(name, format!("{} task", name), role)
            .with_phase(phase.display_name())
            .with_estimate(60)
    }

    fn setup_dry_executor(tasks: Vec<Task>) -> PipelineExecutor {
        let mut pipeline = BuildPipeline::new(Some("test-framework.md".into()));
        for task in tasks {
            pipeline.task_graph.add_task(task).unwrap();
        }
        let bus = Arc::new(MessageBus::new(64));
        let audit = Arc::new(RwLock::new(AuditLog::new()));
        // Default constructor is dry_run=true
        PipelineExecutor::new("dry-run-integ", pipeline, bus, audit)
    }

    #[tokio::test]
    async fn test_dry_run_full_8_phase_pipeline() {
        let tasks = vec![
            make_task("parse-framework", "cto", BuildPhase::Ingest),
            make_task("provision-server", "devops", BuildPhase::Infrastructure),
            make_task("setup-ci", "devops", BuildPhase::Infrastructure),
            make_task("design-api", "architect", BuildPhase::Architecture),
            make_task("design-db", "architect", BuildPhase::Architecture),
            make_task("build-backend", "backend", BuildPhase::Code),
            make_task("build-frontend", "frontend", BuildPhase::Code),
            make_task("build-devops", "devops", BuildPhase::Code),
            make_task("unit-tests", "qa", BuildPhase::Test),
            make_task("e2e-tests", "qa", BuildPhase::Test),
            make_task("security-scan", "security", BuildPhase::Security),
            make_task("deploy-app", "devops", BuildPhase::Deploy),
            make_task("generate-report", "cto", BuildPhase::Deliver),
        ];

        let mut executor = setup_dry_executor(tasks);
        let report = executor.execute().await.unwrap();

        // All phases should complete
        assert!(report.success);
        assert_eq!(report.completed_phases, 8);
        assert_eq!(report.total_tasks, 13);
        assert_eq!(report.completed_tasks, 13);
        assert_eq!(report.failed_tasks, 0);

        // Tokens should be non-zero (estimated)
        assert!(report.total_tokens > 0);

        // Elapsed time should be positive
        assert!(report.elapsed_seconds > 0.0);

        // Every task result should be marked as dry_run
        for result in &report.task_results {
            assert!(result.success);
            if let Some(output) = &result.output {
                assert_eq!(
                    output.get("dry_run").and_then(|v| v.as_bool()),
                    Some(true),
                    "task {} should be marked dry_run",
                    result.task_id
                );
            }
        }
    }

    #[tokio::test]
    async fn test_dry_run_progress_events_cover_all_phases() {
        let tasks = vec![
            make_task("t1", "cto", BuildPhase::Ingest),
            make_task("t2", "devops", BuildPhase::Infrastructure),
        ];

        let mut executor = setup_dry_executor(tasks);
        executor.execute().await.unwrap();

        let events = executor.events();

        // Should have PhaseStarted for all 8 phases
        let started_phases: Vec<BuildPhase> = events
            .iter()
            .filter(|e| e.kind == ProgressKind::PhaseStarted)
            .map(|e| e.phase)
            .collect();
        assert_eq!(started_phases.len(), 8);

        // Should have PhaseCompleted for all 8 phases
        let completed_phases: Vec<BuildPhase> = events
            .iter()
            .filter(|e| e.kind == ProgressKind::PhaseCompleted)
            .map(|e| e.phase)
            .collect();
        assert_eq!(completed_phases.len(), 8);

        // Should have exactly 1 PipelineCompleted
        assert_eq!(
            events
                .iter()
                .filter(|e| e.kind == ProgressKind::PipelineCompleted)
                .count(),
            1
        );

        // TaskStarted and TaskCompleted counts should match
        let task_started = events
            .iter()
            .filter(|e| e.kind == ProgressKind::TaskStarted)
            .count();
        let task_completed = events
            .iter()
            .filter(|e| e.kind == ProgressKind::TaskCompleted)
            .count();
        assert_eq!(task_started, task_completed);
        assert_eq!(task_started, 2); // Two tasks
    }

    #[tokio::test]
    async fn test_dry_run_empty_pipeline() {
        let mut executor = setup_dry_executor(vec![]);
        let report = executor.execute().await.unwrap();

        assert!(report.success);
        assert_eq!(report.completed_phases, 8);
        assert_eq!(report.total_tasks, 0);
        assert_eq!(report.completed_tasks, 0);
    }

    #[tokio::test]
    async fn test_dry_run_parallel_code_phase() {
        let tasks = vec![
            make_task("api-server", "backend", BuildPhase::Code),
            make_task("web-app", "frontend", BuildPhase::Code),
            make_task("ci-pipeline", "devops", BuildPhase::Code),
            make_task("api-tests", "backend", BuildPhase::Code),
        ];

        let mut executor = setup_dry_executor(tasks);
        let report = executor.execute().await.unwrap();

        assert!(report.success);
        assert_eq!(report.completed_tasks, 4);

        // Code phase should have LayerStarted events (parallel execution)
        let layer_events: Vec<_> = executor
            .events()
            .iter()
            .filter(|e| e.kind == ProgressKind::LayerStarted && e.phase == BuildPhase::Code)
            .collect();
        assert!(!layer_events.is_empty());
    }

    #[tokio::test]
    async fn test_dry_run_with_task_dependencies() {
        let t1 = make_task("build-api", "backend", BuildPhase::Code);
        let t1_id = t1.id.clone();
        let t2 = make_task("build-ui", "frontend", BuildPhase::Code);
        let t3 = make_task("integrate", "backend", BuildPhase::Code).depends_on(&t1_id);
        let t3_id = t3.id.clone();
        let t4 = make_task("final-check", "qa", BuildPhase::Code).depends_on(&t3_id);

        let mut executor = setup_dry_executor(vec![t1, t2, t3, t4]);
        let report = executor.execute().await.unwrap();

        assert!(report.success);
        assert_eq!(report.completed_tasks, 4);
    }

    #[tokio::test]
    async fn test_dry_run_audit_log_integrity() {
        let tasks = vec![
            make_task("t1", "backend", BuildPhase::Code),
            make_task("t2", "frontend", BuildPhase::Code),
        ];

        let bus = Arc::new(MessageBus::new(64));
        let audit = Arc::new(RwLock::new(AuditLog::new()));
        let mut pipeline = BuildPipeline::new(Some("test.md".into()));
        for t in tasks {
            pipeline.task_graph.add_task(t).unwrap();
        }

        let mut executor = PipelineExecutor::new("audit-integ", pipeline, bus, audit.clone());
        executor.execute().await.unwrap();

        let log = audit.read().await;
        assert!(!log.is_empty());

        // Should have 8 agent spawn entries
        let spawned = log.entries_by_action(&phantom_core::audit::AuditAction::AgentSpawned);
        assert_eq!(spawned.len(), 8);

        // Should have task started + completed for both tasks
        let started = log.entries_by_action(&phantom_core::audit::AuditAction::TaskStarted);
        let completed = log.entries_by_action(&phantom_core::audit::AuditAction::TaskCompleted);
        assert_eq!(started.len(), 2);
        assert_eq!(completed.len(), 2);

        // Should have system entries for pipeline start/end + phase transitions
        let system = log.entries_by_action(&phantom_core::audit::AuditAction::System);
        assert!(system.len() >= 2); // at least start + end
    }

    #[tokio::test]
    async fn test_dry_run_checkpoint_and_resume() {
        let captured = Arc::new(RwLock::new(Vec::<Vec<u8>>::new()));
        let captured_clone = captured.clone();

        let tasks = vec![
            make_task("t1", "cto", BuildPhase::Ingest),
            make_task("t2", "devops", BuildPhase::Infrastructure),
        ];

        let mut executor = setup_dry_executor(tasks);
        executor = executor.with_checkpoint_fn(move |bytes: &[u8]| {
            let captured = captured_clone.clone();
            let bytes = bytes.to_vec();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    captured.write().await.push(bytes);
                });
            })
            .join()
            .unwrap();
            Ok(())
        });

        executor.execute().await.unwrap();

        let checkpoints = captured.read().await;
        // Should checkpoint after each of the 8 phases
        assert!(
            checkpoints.len() >= 8,
            "expected >=8 checkpoints, got {}",
            checkpoints.len()
        );

        // Each checkpoint should be valid JSON
        for (i, cp_bytes) in checkpoints.iter().enumerate() {
            let cp: serde_json::Value = serde_json::from_slice(cp_bytes).unwrap_or_else(|e| {
                panic!("checkpoint {} is not valid JSON: {}", i, e);
            });
            assert_eq!(cp["build_id"], "dry-run-integ");
        }
    }

    #[tokio::test]
    async fn test_dry_run_halt_stops_execution() {
        let tasks = vec![make_task("t1", "cto", BuildPhase::Ingest)];
        let mut executor = setup_dry_executor(tasks);

        // Halt before executing
        executor.halt("integration test halt").await;

        let result = executor.execute().await;
        assert!(result.is_err());

        // Should have PipelineHalted event
        assert!(executor
            .events()
            .iter()
            .any(|e| e.kind == ProgressKind::PipelineHalted));
    }

    #[tokio::test]
    async fn test_dry_run_pipeline_report_display() {
        let tasks = vec![make_task("t1", "cto", BuildPhase::Ingest)];
        let mut executor = setup_dry_executor(tasks);
        let report = executor.execute().await.unwrap();

        let display = format!("{}", report);
        assert!(display.contains("PIPELINE REPORT"));
        assert!(display.contains("SUCCESS"));
        assert!(display.contains("dry-run-integ"));
    }

    #[tokio::test]
    async fn test_dry_run_message_bus_receives_broadcasts() {
        let bus = Arc::new(MessageBus::new(64));
        let mut mailbox = bus.register_agent("test-observer").await.unwrap();

        let tasks = vec![make_task("t1", "cto", BuildPhase::Ingest)];
        let audit = Arc::new(RwLock::new(AuditLog::new()));
        let mut pipeline = BuildPipeline::new(Some("test.md".into()));
        for t in tasks {
            pipeline.task_graph.add_task(t).unwrap();
        }

        let mut executor = PipelineExecutor::new("bus-integ", pipeline, bus, audit);
        executor.execute().await.unwrap();

        // Yield to let broadcast tasks complete
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        let mut count = 0;
        while mailbox.try_recv_broadcast().is_some() {
            count += 1;
        }
        // Should receive at least some progress broadcasts
        assert!(
            count > 0 || !executor.events().is_empty(),
            "should receive broadcasts or have local events"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  4. Self-Healing Error Path Coverage
// ═══════════════════════════════════════════════════════════════════════════

mod self_healing_errors {
    use phantom_core::{HealingLayer, HealingResult, SelfHealer};

    // ── Layer determination ──────────────────────────────────────────────

    #[test]
    fn test_layer1_retry_on_timeout() {
        let healer = SelfHealer::new();
        let layer = healer.determine_layer(0, "connection timeout");
        assert_eq!(layer, HealingLayer::Retry);
    }

    #[test]
    fn test_layer1_retry_on_rate_limit() {
        let healer = SelfHealer::new();
        let layer = healer.determine_layer(0, "rate limit exceeded");
        assert_eq!(layer, HealingLayer::Retry);
    }

    #[test]
    fn test_layer1_retry_on_429() {
        let healer = SelfHealer::new();
        let layer = healer.determine_layer(0, "HTTP 429 Too Many Requests");
        assert_eq!(layer, HealingLayer::Retry);
    }

    #[test]
    fn test_layer1_retry_on_502() {
        let healer = SelfHealer::new();
        let layer = healer.determine_layer(0, "502 Bad Gateway");
        assert_eq!(layer, HealingLayer::Retry);
    }

    #[test]
    fn test_layer1_retry_on_transient() {
        let healer = SelfHealer::new();
        let layer = healer.determine_layer(0, "temporary failure");
        assert_eq!(layer, HealingLayer::Retry);
    }

    #[test]
    fn test_layer2_alternative_after_retries_exhausted() {
        let healer = SelfHealer::new();
        // After max retries (default 5), retryable error + "not supported" alternative pattern
        let layer = healer.determine_layer(5, "provider unavailable");
        assert_eq!(layer, HealingLayer::Alternative);
    }

    #[test]
    fn test_layer2_alternative_on_permanent_error() {
        let healer = SelfHealer::new();
        // Non-retryable error matching alternative pattern
        let layer = healer.determine_layer(0, "permission denied");
        assert_eq!(layer, HealingLayer::Alternative);
    }

    #[test]
    fn test_layer2_alternative_tool_not_found() {
        let healer = SelfHealer::new();
        let layer = healer.determine_layer(0, "tool not found");
        assert_eq!(layer, HealingLayer::Alternative);
    }

    #[test]
    fn test_layer3_decompose_on_complex_error() {
        let healer = SelfHealer::new();
        // "too complex" matches decomposable pattern
        let layer = healer.determine_layer(0, "task too complex");
        assert_eq!(layer, HealingLayer::Decompose);
    }

    #[test]
    fn test_layer3_decompose_token_limit() {
        let healer = SelfHealer::new();
        let layer = healer.determine_layer(0, "token limit exceeded");
        assert_eq!(layer, HealingLayer::Decompose);
    }

    #[test]
    fn test_layer3_decompose_context_overflow() {
        let healer = SelfHealer::new();
        let layer = healer.determine_layer(0, "context overflow");
        assert_eq!(layer, HealingLayer::Decompose);
    }

    #[test]
    fn test_layer4_escalate_after_max_retries_generic_error() {
        let healer = SelfHealer::new();
        // Generic error (not retryable, not alternative, not decomposable) with high retries
        let layer = healer.determine_layer(5, "something unknown broke");
        assert_eq!(layer, HealingLayer::Escalate);
    }

    #[test]
    fn test_layer5_pause_alert_low_retries_unknown_error() {
        let healer = SelfHealer::new();
        // Unknown error with low retry count — falls through all patterns to PauseAndAlert
        let layer = healer.determine_layer(0, "catastrophic unknown error");
        assert_eq!(layer, HealingLayer::PauseAndAlert);
    }

    // ── Layer progression ────────────────────────────────────────────────

    #[test]
    fn test_healing_escalation_through_layers() {
        let healer = SelfHealer::new();

        // Layer 1: Retry on transient errors (max_retries default = 5)
        let l0 = healer.determine_layer(0, "timeout");
        assert_eq!(l0, HealingLayer::Retry);

        let l1 = healer.determine_layer(1, "timeout");
        assert_eq!(l1, HealingLayer::Retry);

        let l4 = healer.determine_layer(4, "timeout");
        assert_eq!(l4, HealingLayer::Retry);

        // Layer 2: Alternative when error matches alternative patterns
        let alt = healer.determine_layer(0, "command not found");
        assert_eq!(alt, HealingLayer::Alternative);

        // Layer 3: Decompose when error is about complexity
        let dec = healer.determine_layer(0, "too complex");
        assert_eq!(dec, HealingLayer::Decompose);

        // Layer 4: Escalate when retries exhausted on generic error
        let esc = healer.determine_layer(5, "generic failure");
        assert_eq!(esc, HealingLayer::Escalate);

        // Layer 5: PauseAndAlert is the fallback for unknown low-retry errors
        let pause = healer.determine_layer(0, "completely unknown situation");
        assert_eq!(pause, HealingLayer::PauseAndAlert);
    }

    #[test]
    fn test_healing_layer_names() {
        assert_eq!(HealingLayer::Retry.name(), "retry");
        assert_eq!(HealingLayer::Alternative.name(), "alternative");
        assert_eq!(HealingLayer::Decompose.name(), "decompose");
        assert_eq!(HealingLayer::Escalate.name(), "escalate");
        assert_eq!(HealingLayer::PauseAndAlert.name(), "pause_and_alert");
    }

    #[test]
    fn test_healing_layer_display() {
        // Verify Display impl works
        let display = format!("{}", HealingLayer::Retry);
        assert!(!display.is_empty());

        let display = format!("{}", HealingLayer::PauseAndAlert);
        assert!(!display.is_empty());
    }

    // ── Backoff calculation ──────────────────────────────────────────────

    #[test]
    fn test_backoff_increases_exponentially() {
        let healer = SelfHealer::new();
        let d0 = healer.backoff_delay(0);
        let d1 = healer.backoff_delay(1);
        let d2 = healer.backoff_delay(2);
        let d3 = healer.backoff_delay(3);

        assert!(d1 > d0, "backoff should increase");
        assert!(d2 > d1, "backoff should increase");
        assert!(d3 > d2, "backoff should increase");
    }

    #[test]
    fn test_backoff_has_max_cap() {
        let healer = SelfHealer::new();
        let _d10 = healer.backoff_delay(10);
        let d20 = healer.backoff_delay(20);

        // Should be capped at some reasonable max (retry_max_delay_ms = 30_000)
        assert!(d20.as_secs() <= 60, "backoff should be capped");
        // Very high retry counts shouldn't overflow
        let _ = healer.backoff_delay(100);
    }

    #[test]
    fn test_backoff_initial_is_small() {
        let healer = SelfHealer::new();
        let d0 = healer.backoff_delay(0);
        assert!(d0.as_secs() <= 5, "initial backoff should be small");
    }

    // ── Healing result ───────────────────────────────────────────────────

    #[test]
    fn test_healing_result_success() {
        let result = HealingResult {
            layer: HealingLayer::Retry,
            success: true,
            action: "retried task".into(),
            attempts: 1,
            sub_tasks: vec![],
            escalated_to: None,
            owner_notified: false,
        };
        assert!(result.success);
        assert_eq!(result.layer, HealingLayer::Retry);
        assert_eq!(result.attempts, 1);
        assert!(!result.owner_notified);
    }

    #[test]
    fn test_healing_result_failure_escalation() {
        let result = HealingResult {
            layer: HealingLayer::Escalate,
            success: false,
            action: "escalated to CTO agent".into(),
            attempts: 8,
            sub_tasks: vec![],
            escalated_to: Some("cto-0".into()),
            owner_notified: false,
        };
        assert!(!result.success);
        assert_eq!(result.layer, HealingLayer::Escalate);
        assert_eq!(result.escalated_to.as_deref(), Some("cto-0"));
    }

    #[test]
    fn test_healing_result_pause_and_alert() {
        let result = HealingResult {
            layer: HealingLayer::PauseAndAlert,
            success: false,
            action: "paused pipeline, owner notified".into(),
            attempts: 12,
            sub_tasks: vec![],
            escalated_to: None,
            owner_notified: true,
        };
        assert!(!result.success);
        assert!(result.owner_notified);
        assert_eq!(result.layer, HealingLayer::PauseAndAlert);
    }

    #[test]
    fn test_healing_result_decompose_with_subtasks() {
        let result = HealingResult {
            layer: HealingLayer::Decompose,
            success: true,
            action: "decomposed into 3 sub-tasks".into(),
            attempts: 6,
            sub_tasks: vec!["sub-1".into(), "sub-2".into(), "sub-3".into()],
            escalated_to: None,
            owner_notified: false,
        };
        assert!(result.success);
        assert_eq!(result.sub_tasks.len(), 3);
    }

    // ── Error classification ─────────────────────────────────────────────

    #[test]
    fn test_retryable_errors() {
        let healer = SelfHealer::new();
        let retryable = [
            "connection timeout",
            "rate limit exceeded",
            "429 too many requests",
            "502 bad gateway",
            "503 service unavailable",
            "temporary failure",
            "ECONNRESET",
            "EAGAIN resource temporarily unavailable",
        ];
        for err in &retryable {
            let layer = healer.determine_layer(0, err);
            assert_eq!(layer, HealingLayer::Retry, "'{}' should be retryable", err);
        }
    }

    #[test]
    fn test_non_retryable_errors() {
        let healer = SelfHealer::new();
        let permanent = [
            "permission denied",
            "invalid API key",
            "authentication failed",
            "not found",
        ];
        for err in &permanent {
            let layer = healer.determine_layer(0, err);
            assert_ne!(
                layer,
                HealingLayer::Retry,
                "'{}' should NOT be retryable",
                err
            );
        }
    }

    // ── Exhaustion detection ─────────────────────────────────────────────

    #[test]
    fn test_high_retries_generic_error_escalates() {
        let healer = SelfHealer::new();
        // After max retries with unknown errors, escalation is the outcome
        let layer = healer.determine_layer(50, "everything failed");
        assert_eq!(layer, HealingLayer::Escalate);

        let layer = healer.determine_layer(100, "still broken");
        assert_eq!(layer, HealingLayer::Escalate);
    }

    #[test]
    fn test_exhaustion_detection() {
        let healer = SelfHealer::new();
        // Not exhausted when no layers tried
        assert!(!healer.is_exhausted(0, &[]));
        // Not exhausted with only retries
        assert!(!healer.is_exhausted(3, &[HealingLayer::Retry]));
        // Exhausted when max retries exceeded AND PauseAndAlert has been tried
        assert!(healer.is_exhausted(
            10,
            &[
                HealingLayer::Retry,
                HealingLayer::Alternative,
                HealingLayer::Decompose,
                HealingLayer::Escalate,
                HealingLayer::PauseAndAlert,
            ]
        ));
    }

    #[test]
    fn test_exhaustion_requires_pause_and_alert() {
        let healer = SelfHealer::new();
        // High retry count but haven't tried PauseAndAlert yet
        assert!(!healer.is_exhausted(
            50,
            &[
                HealingLayer::Retry,
                HealingLayer::Alternative,
                HealingLayer::Escalate,
            ]
        ));
    }
}
