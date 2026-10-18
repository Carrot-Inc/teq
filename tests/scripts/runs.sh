# The runs of tests/interp-script.sh: `run <name> <script> <argument>...`, each against <name>.expected (its
# stdout), and <name>.status and <name>.stderr (the first line of its stderr) where its status is not 0, which
# Scala CLI 1.17.1's run of the script under scalac 3.8.4 wrote (`tests/interp-script.sh --record`).
run echo-args echo-args.sc a b 'c d'
run echo-args-none echo-args.sc
run uses-defs uses-defs.sc one two
run reads-other-args reads-other-args.sc x
run main-object main-object.sc p q
run exit-status-0 exit-status.sc
run exit-status-3 exit-status.sc 3
run exit-status-fail exit-status.sc fail
run nested-main nested-main.sc
run nested-app nested-app.sc
run backticked-main backticked-main.sc
run parenthesised-main parenthesised-main.sc
run_dir dirs-package dirs main_sc
run_dir script-paths-dir paths main_sc
run_dir hyphen-main hyphen 'x$minusy_sc'
run_dir hyphen-package-main hyphen-pkg 'x$minusy.run_sc'
run_dir hyphen-mirror mirror 'x$minusy_sc'
run hyphen-object-reference hyphen-ref/x-y.sc
run_dir hyphen-package-reference hyphen-pkg-ref main_sc
run_dir hyphen-package-raw-reference hyphen-pkg-raw main_sc
run_dir hyphen-package-merge hyphen-merge 'x$minusy.run_sc'
run script-paths script-paths.sc
refusal_cp queue-version queue-version.scala dev/zio/zio_3/2.1.25/zio_3-2.1.25.jar dev/zio/zio-internal-macros_3/2.1.25/zio-internal-macros_3-2.1.25.jar dev/zio/zio-stacktracer_3/2.1.25/zio-stacktracer_3-2.1.25.jar dev/zio/izumi-reflect_3/3.0.9/izumi-reflect_3-3.0.9.jar dev/zio/izumi-reflect-thirdparty-boopickle-shaded_3/3.0.9/izumi-reflect-thirdparty-boopickle-shaded_3-3.0.9.jar org/scala-lang/modules/scala-collection-compat_3/2.14.0/scala-collection-compat_3-2.14.0.jar
