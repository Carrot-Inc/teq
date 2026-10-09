package errs

/** An error in a signature written out: resolved once. */
class A:
  def f(x: Missing): Int = 1
  def label: String = "a"
