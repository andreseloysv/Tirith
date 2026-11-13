case " $* " in *" -IDEPackageSupportDisableManifestSandbox"*) exec xcodebuild "$@";; esac
case " $* " in *" -scmProvider "*) scm=;; *) scm='-scmProvider system';; esac
case " $* " in *" OTHER_SWIFT_FLAGS="*) exec xcodebuild {flags} $scm "$@";; esac
exec xcodebuild {flags} $scm 'OTHER_SWIFT_FLAGS=$(inherited) -disable-sandbox' "$@"
