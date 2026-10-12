// `p.C(args)` for a class nested in the class of `p`: a case class's companion `apply`, a plain
// class's constructor proxy, an explicit companion without `apply`, all with `p` as the
// enclosing instance.
trait Framework:
  def tag: String
  case class Match(q: Int):
    def show = s"$tag:Match($q)"
  class Project(val x: Int):
    def show = s"$tag:Project($x)"
  case class Pair(a: Int, b: Int):
    def show = s"$tag:Pair($a,$b)"
  object Pair:
    def one(a: Int) = Pair(a, 1)
  case class Box[A](a: A):
    def show = s"$tag:Box($a)"

class Coll(name: String):
  object framework extends Framework:
    def tag = name

class Plain(val tag: String) extends Framework

var made = 0
def counted(fw: Framework): Framework =
  made += 1
  fw

def run(fw: Framework) =
  val m = fw.Match(1)
  val p = fw.Project(2)
  val q = fw.Pair(3, 4)
  val r = fw.Pair.one(5)
  val b = fw.Box("x")
  val s = m match
    case fw.Match(q) => q + 10
  val t = counted(fw).Match(9)
  List(m.show, p.show, q.show, r.show, b.show, s.toString, t.show)

@main def m() =
  println(run(Coll("a").framework))
  println(run(Plain("b")))
  val c = Coll("c")
  println(c.framework.Match(3).show)
  println(c.framework.Project(4).show)
  println(made)
