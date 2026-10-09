package ann

class Uses:
  @Child def f: Int = 1
  @Body def g: Int = 2
  @Tag(classOf[String]) def s: Int = 3
  @Tag(classOf[Int]) def i: Int = 4
  @deprecated("use f", "1.0") def old: Int = 5
