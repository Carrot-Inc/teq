//> using dep org.reactivemongo::reactivemongo-bson-api:1.1.0-noshaded.RC21
//> using platform jvm
// jars: reactivemongo-bson-api
// A library's class named by a character that is no part of a Java identifier lies in its jar under
// the code of the character (`$u00AC.tasty` for reactivemongo's `¬`, as NameTransformer.encode spells
// it): its companion's evidence takes part in the search of reactivemongo's collection writer.
import reactivemongo.api.bson.*

@main def run(): Unit =
  println(summon[List[String] ¬ Option[String]])
  println(summon[BSONWriter[List[String]]].writeTry(List("a", "b")).map(_.toString).isSuccess)
