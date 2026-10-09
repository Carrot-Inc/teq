package errs

/** An error in the body of a val whose type is inferred: the retype infers it again. */
object I:
  val broken = 1.nothing
  def label: String = "i"

val brokenTop = 2.nothing

def brokenDef = 3.nothing
