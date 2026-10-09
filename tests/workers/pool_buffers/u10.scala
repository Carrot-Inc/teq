import izumi.reflect.Tag
object U10:
  val t: String = Tag[Big[Option[List[Map[String, Big[Int]]]]]].tag.toString + 10
