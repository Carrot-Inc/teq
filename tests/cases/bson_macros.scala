//> using dep org.reactivemongo::reactivemongo-bson-api:1.1.0-noshaded.RC21
//> using platform jvm
// jars: reactivemongo-bson-api
// split-watch: jvm
// reactivemongo's Macros.handler, reader and writer, whose implementation (pickled by Scala 3.3) binds
// `tt @ '[t]` as a `Type[t]`, matches `'[IsProduct[t]]` under a local `type IsProduct[U <: Product] = U`,
// lists a product's fields through a `Tuple3 <: Product` mirror and its `_1`..`_n` selectors, and builds
// the document through `lazyZip` and a `LazyList` taken as a `Seq`. A JVM library: the interpreter and
// the JVM target run it, a JVM build's macro reaching `java.time.ZoneId.systemDefault` through the
// package object's handlers.
import reactivemongo.api.bson.*

case class Person(name: String, age: Int, nick: Option[String])
case class Team(title: String, lead: Person, size: Option[Int])

object Main:
  given BSONDocumentHandler[Person] = Macros.handler[Person]
  given BSONDocumentWriter[Team] = Macros.writer[Team]
  given BSONDocumentReader[Team] = Macros.reader[Team]

  def main(args: Array[String]): Unit =
    val p = Person("a", 3, None)
    val doc = BSON.writeDocument(p).get
    println(BSONDocument.pretty(doc))
    println(doc.asTry[Person])
    val q = Person("b", 4, Some("bee"))
    println(BSONDocument.pretty(BSON.writeDocument(q).get))
    val t = Team("t", q, Some(2))
    val tdoc = BSON.writeDocument(t).get
    println(BSONDocument.pretty(tdoc))
    println(tdoc.asTry[Team])
    println(BSONDocument("name" -> "c").asTry[Person].isFailure)
