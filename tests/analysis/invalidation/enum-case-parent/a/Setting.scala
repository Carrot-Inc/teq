package ecp

enum Setting[T](val key: String):
  case A extends Setting[String](Keys.a)
  case B extends Setting[Int]("b")
