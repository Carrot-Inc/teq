//> using dep org.reactivemongo::reactivemongo-bson-api:1.1.0-noshaded.RC21
//> using platform jvm
// jars: reactivemongo-bson-api
// split-watch: jvm
// reactivemongo's Macros.handler of a case class holding a list of opaque ids, its collection writer found
// with an opaque writer of lila's shape (given through an object's parent trait and an import of its
// givens); the interpreter's IndexedSeq.newBuilder builds the array.
import reactivemongo.api.bson.*

trait SameRuntime[A, B]:
  def apply(a: A): B
abstract class TotalWrapper[N, I](using N =:= I):
  given SameRuntime[N, I] = _.asInstanceOf[I]
  given SameRuntime[I, N] = _.asInstanceOf[N]
  def apply(i: I): N = i.asInstanceOf[N]

object ids:
  opaque type GameId = String
  object GameId extends TotalWrapper[GameId, String]

trait NoBSONWriter[A]
trait Handlers:
  given opaqueWriter[T, A](using rs: SameRuntime[T, A], writer: BSONWriter[A])(using scala.util.NotGiven[NoBSONWriter[T]]): BSONWriter[T] with
    def writeTry(t: T) = writer.writeTry(rs(t))
  given opaqueReader[T, A](using sr: SameRuntime[A, T], reader: BSONReader[A]): BSONReader[T] with
    def readTry(bson: BSONValue) = reader.readTry(bson).map(sr.apply)
trait dsl
object dsl extends dsl with Handlers

import ids.*

object BSONHandlers:
  import dsl.{ *, given }
  case class Corres(moves: Int, movesIn: List[GameId])
  given BSONDocumentHandler[Corres] = Macros.handler

@main def run(): Unit =
  import BSONHandlers.{ *, given }
  println(BSONDocument.pretty(summon[BSONDocumentHandler[Corres]].writeTry(Corres(2, List(GameId("g1")))).get))
