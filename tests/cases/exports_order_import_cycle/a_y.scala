// Finding `Z` while `Y`'s header is completed searches `import O.*`, whose object extends a
// class that extends `Y`: scalac accepts this, so the table of `O` cannot force `O`'s
// completion there, in whatever order the files come.
import O.*

class Y extends Z:
  def y: Int = 1
