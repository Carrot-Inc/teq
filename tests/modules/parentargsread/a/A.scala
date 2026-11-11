// The parent call `DBase()` whose default argument the typer fills, and `Ctx2` whose using clause it fills, are no
// `Apply` with arguments in the source (`parentsKind`, TreeInfo.scala:346-350), so `DSub` and `FSub` run nothing and
// the creations of `D5` and `D6` are pure in every build. `Arg`'s written argument runs in the source's judgement,
// which the whole build keeps (`ignore(new D7)` prints `arg`). The build over the products reads the templates
// without their parents' arguments (`readParents(withArgs = false)`, TreeUnpickler.scala:1084-1088), so `Arg` runs
// nothing there and the creation of `D7` is dropped. The split build's output differs from the whole build's in
// scalac 3.8.4 too: `| | |` against `| | arg |`. run.txt holds the whole build's.
package rpa

object Log:
  def note(s: String): Int =
    print(s.concat(" "))
    s.length

class DBase(x: Int = Log.note("dflt"))
class DSub extends DBase

given String = "given"
class Ctx2(using s: String)
class FSub extends Ctx2

class Arg extends DBase(Log.note("arg"))
