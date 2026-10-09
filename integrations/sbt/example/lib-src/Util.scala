package util

// A jar dependency of the api project's tests (check.sh packages it into api/lib/util.jar with
// scala-cli, then again with another value): the constant is inlined, so a suite reading it
// sees a rebuilt jar only when its resident is started afresh.
object Util:
  inline def value: Int = 1
