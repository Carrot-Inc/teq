// A broken header with no result type and an unmatched `[` at the end of the file: what is left
// of the line is skipped with the header, to the end of the file whatever is open.
object O:
  val ok: String = 1
  def h(x: Int) Int [
