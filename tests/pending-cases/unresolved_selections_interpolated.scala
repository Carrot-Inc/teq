// An unresolved import's names spliced into an interpolation; once a crash (apply.rs is_stable_expr). Pending: teq's
// "cannot resolve import: b not found" against scalac's "Not found: b" (the not-found message, NotFoundMsg, the typer's
// messages' owner).
package a
import b.*
def show: String = s"$tag/${BConst.value}"
