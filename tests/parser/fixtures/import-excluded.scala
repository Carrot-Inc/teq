// An unresolved wildcard import leaves the names its clause excludes out of what it could bind:
// `gone` is not found, beside the import that does not resolve.
import Missing.{gone as _, *}
object O:
  val x = gone
