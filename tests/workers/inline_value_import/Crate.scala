// An inline method whose body imports through a value (a given of the value's object and one of its methods): each
// expansion walks the import into the copy, an entry of the table of value imports every worker shares. Sixty-four
// calls in eight files: the forked inlining phase's workers push to the table at once.
package crates

class Badge(val words: String)

class Crate(val code: String):
  object tray:
    given badge: Badge = new Badge(code)
    def doubled: String = code + code

object Kit:
  inline def open(code: String): String =
    val cr = new Crate(code)
    import cr.tray.given
    import cr.tray.doubled
    scala.compiletime.summonInline[Badge].words + doubled
  inline def openGiven(c: Crate): String =
    import c.tray.given
    import c.tray.doubled
    scala.compiletime.summonInline[Badge].words + doubled
