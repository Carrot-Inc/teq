// jars: scala-library chimney-jvm chimney-macro-commons-jvm scala-collection-compat-jvm
// std: scala-library
//> using dep io.scalaland::chimney:1.11.0
// chimney's derivation between case classes whose field is a `List` of case classes, in link
// mode: the elements' transformer is derived and the list rebuilt through `TotallyBuildIterable`.
import io.scalaland.chimney.dsl.*

final case class FieldInfo(name: String, fieldType: String, facet: Option[Boolean], optional: Option[Boolean], index: Option[Boolean])
final case class CollectionInfo(name: String, numDocuments: Long, createdAt: Long, fields: List[FieldInfo])
final case class IndexField(name: String, fieldType: String, facet: Option[Boolean], optional: Option[Boolean], index: Option[Boolean])
final case class IndexCollection(name: String, numDocuments: Long, createdAt: Long, fields: List[IndexField])

def convert(infos: List[CollectionInfo]): List[IndexCollection] = infos.map(_.transformInto[IndexCollection])

@main def run(): Unit =
  val info = CollectionInfo("products", 12L, 7L, List(FieldInfo("id", "string", None, Some(false), Some(true)), FieldInfo("price", "int32", Some(true), None, None)))
  println(convert(List(info)))
