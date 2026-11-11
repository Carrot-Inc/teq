// A class whose parent takes arguments that run code runs them when an instance is made: its constructor is not
// stable (`setStableConstructor`, SymDenotations.scala:97-99, the parents' kind of its template, Namer.scala:1791).
// A downstream reads that from the constructor's pickled flag (`isPureApply`, TreeInfo.scala:673), the parents'
// arguments being no more in the template it reads (`readParents(withArgs = false)`, TreeUnpickler.scala:1084-1088).
// (A case class's `apply` is decided by `isNoInitsRealClass` over that template, `isCaseClassApply`, Inliner.scala:
// 66-75: scalac's own separate build drops `ignore(QC(1))` written through an import and keeps `ignore(spa.QC(1))`,
// where its joint build keeps both: not here.)
package spa

object Log:
  def note(s: String): Int =
    print(s.concat(" "))
    s.length

class Base(x: Int)

class Q extends Base(Log.note("q"))

trait TB(val t: Int)

class QT extends TB(Log.note("qt"))


class Plain(x: Int) extends Base(x)
