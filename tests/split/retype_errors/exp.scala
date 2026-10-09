package errs

object Source:
  def given1: Int = 1

export Source.gone

/** Export clauses that name nothing, the file's and a class's. */
object Exp:
  export Source.missing
  def label: String = "e"
