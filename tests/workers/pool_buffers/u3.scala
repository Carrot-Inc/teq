import izumi.reflect.Tag
object U3:
  val t: String = Tag[Big[Option[List[Map[String, Big[Int]]]]]].tag.toString + 3
