// The base type of a class for an ancestor its parents give at several instantiations is their meet, as dotty's
// `baseTypeOf` folds the parents' (`foldGlb`, `TypeComparer.glb`'s `glbArgs`): a covariant constructor argument
// the narrower (`Base[Sig, A]` over `Base[Obs, A]`, so `map` gives a `Sig`), a covariant proper argument the
// narrower or the intersection of the two, a contravariant one the wider; a subclass named in a parent's arguments,
// not complete when the class is (zio's `ChunkLike[A] extends IndexedSeq[A] with IndexedSeqOps[A, Chunk, Chunk[A]]`),
// taken from the parent written later, which is the meet.
trait Base[+Self[+_] <: Obs[_], +A]:
  def map[B](f: A => B): Self[B]
trait Obs[+A] extends Base[Obs, A]
trait Sig[+A] extends Obs[A] with Base[Sig, A]:
  def now(): A
trait Strict[+A] extends Sig[A] with Base[Sig, A]

class Cell[+A](v: A) extends Strict[A]:
  def map[B](f: A => B): Sig[B] = new Cell(f(v))
  def now(): A = v

trait Named { def name: String = "named" }
trait Sized { def size: Int = 3 }
trait Holder[+T] { def held: T }
trait NamedHolder extends Holder[Named]
trait SizedHolder extends Holder[Sized]
class Both extends NamedHolder with SizedHolder:
  def held: Named & Sized = new Named with Sized

trait Sink[-T] { def put(t: T): String }
trait IntSink extends Sink[Int]
trait AnySink extends Sink[Any]
class Drain extends IntSink with AnySink:
  def put(t: Any): String = s"put $t"

trait Ops[+A, +CC[_], +C]:
  def mapped: C
trait Coll[+A] extends Ops[A, Coll, Coll[A]]
trait ChunkLike[+A] extends Coll[A] with Ops[A, Chunk, Chunk[A]]
final class Chunk[+A](val items: List[A]) extends ChunkLike[A]:
  def mapped: Chunk[A] = new Chunk(items.reverse)

def remapped(c: Chunk[Int]): Chunk[Int] = c.mapped
def opsOf(c: ChunkLike[Int]): Chunk[Int] = c.mapped

def heldOf(h: Holder[Named & Sized]): String = h.held.name + h.held.size

def through[Self[+_] <: Obs[_], A](b: Base[Self, A]): Self[String] = b.map(_.toString + "!")

@main def main(): Unit =
  val s: Sig[Int] = new Cell(1)
  val t: Sig[String] = s.map(_.toString + "?")
  println(t.now())
  val st: Strict[Int] = new Cell(2)
  println(st.map(_ + 1).now())
  println(through[Sig, Int](s).now())
  val h = (new Both).held
  println(h.name * h.size)
  println(heldOf(new Both))
  val sink: Sink[String] = new Drain
  println(sink.put("x"))
  println(remapped(new Chunk(List(1, 2, 3))).items)
  println(opsOf(new Chunk(List(4, 5))).items)
