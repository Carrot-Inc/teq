package errs

/** A type alias of a type that is not there. */
type Gone = NoSuchType

object Alias:
  def label: String = "alias"
