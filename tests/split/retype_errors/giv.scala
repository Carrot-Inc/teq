package errs

trait Shown[A]:
  def show(a: A): String

/** A given's type and a using clause's that are not there. */
given noGiven: Shown[NoGivenType] with
  def show(a: NoGivenType): String = "g"

def uses(using s: Shown[NoUsing]): String = "u"
