// An unresolved import's names spliced into an interpolation.
// Pending: scalac's "Not found: b" (the not-found
// message, NotFoundMsg, the typer's messages' owner).
package a
import b.*
def show: String = s"$tag/${BConst.value}"
