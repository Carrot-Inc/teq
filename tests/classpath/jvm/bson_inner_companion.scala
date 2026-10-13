// jars: reactivemongo-bson-api
//> using dep org.reactivemongo::reactivemongo-bson-api:1.1.0-noshaded.RC21
// reactivemongo's Macros.handler and Macros.reader over a case class nested in a class and declaring no
// companion (lila's MongoCache.Entry and SessionStore.DedupInfo): the macros take the class's companion,
// `Symbol.companionModule` the made companion's val, a term, and `Ref` of it is that val read from the
// enclosing instance (`tpd.ref`'s `This(owner).select`), as scalac's QuotesImpl answers. The run writes
// documents alone: no instance is made by the macros' readers.
import reactivemongo.api.bson.*

final class Store(tag: String):
  case class Entry(_id: String, v: Int, note: Option[String] = None)
  given BSONDocumentHandler[Entry] = Macros.handler
  def write(e: Entry): String = s"$tag ${BSONDocument.pretty(BSON.writeDocument(e).get)}"

final class Sessions:
  case class DedupInfo(_id: String, n: Int)
  given BSONDocumentReader[DedupInfo] = Macros.reader
  def reads: Boolean = summon[BSONDocumentReader[DedupInfo]] != null

object Holder:
  private case class Alarm(_id: String, n: Int)
  private given BSONDocumentHandler[Alarm] = Macros.handler
  def doc: String = BSONDocument.pretty(BSON.writeDocument(Alarm("a", 1)).get)

@main def run(): Unit =
  val s = Store("one")
  println(s.write(s.Entry("x", 1)))
  println(s.write(s.Entry("y", 2, Some("n"))))
  val t = Store("two")
  println(t.write(t.Entry("z", 3)))
  println(Sessions().reads)
  println(Holder.doc)
