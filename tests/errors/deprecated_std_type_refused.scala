// expect: warning: there was 1 deprecation warning; re-run with --deprecation for details
// expect: 1 warning found, errors under --werror
// teq: --werror
// A type alone names a deprecated definition of the std: its warning is an error of a build that asks for it.
object Alone {
  def size(s: Stream[Int]): Int = 1
}
