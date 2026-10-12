// A package applied as a function calls its top-level `apply`: dotty's checkValue leaves an
// applied package unchecked and tryInsertApplyOrImplicit selects the `apply`.
package views.activity

def apply(n: Int): String = s"activity $n"
def apply(s: String): String = s"activity of $s"
