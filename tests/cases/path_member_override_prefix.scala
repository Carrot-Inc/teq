// A path type written through an enclosing class's `this` (`QueryBuilderFactory.this.pack.Document`
// in a member of an inner class) and seen from a receiver whose class narrows the member
// (`val pack: BPack.type`) takes the member the receiver has under the name, as dotty's
// NamedType takes its denotation anew from the prefix; an overloaded method's alternatives are
// seen from the receiver too before the arguments are tried against them.
trait Pack:
  type Document
object BPack extends Pack:
  type Document = String

trait QueryBuilderFactory[P <: Pack]:
  val pack: P
  final class QueryBuilder(val q: Int):
    def sort(document: pack.Document): QueryBuilder = { println(s"sort $document"); this }
    def sort: Option[pack.Document] = None
    def filter(document: pack.Document): QueryBuilder = { println(s"filter $document"); this }

trait Coll extends QueryBuilderFactory[BPack.type]:
  val pack: BPack.type = BPack
  def find(q: Int): QueryBuilder = new QueryBuilder(q)

class C extends Coll

@main def run(): Unit =
  val c = new C
  println(c.find(1).filter("f").sort("x").q)
  println(c.find(2).sort)
