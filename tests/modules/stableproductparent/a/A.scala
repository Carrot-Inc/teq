// A product class whose parent's constructor call passes arguments initialises, as its source's `parentsKind` has it
// (Namer.scala:1791, TreeInfo.scala:346-350), but not as the template a downstream reads: `readTemplate` skips the
// pickled parents' arguments (`readParents(withArgs = false)`, TreeUnpickler.scala:1084-1088). So the whole build
// keeps `ignore(new D2)` with `Arg`'s parent call's print, and the build over the products, where `D2`'s constructor
// is stable, drops it. The split build's output differs from the whole build's in scalac 3.8.4: its whole build
// prints `arg |`, its build over its own products `|`. run.txt holds the whole build's.
package paa

object Log:
  def note(s: String): Int =
    print(s.concat(" "))
    s.length

class Base(x: Int)

class Arg extends Base(Log.note("arg"))
