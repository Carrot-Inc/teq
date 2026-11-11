object A0 {
  def make[T: scala.reflect.ClassTag](n: Int): Array[T] = new Array[T](n)
  def run: Int = make[String](0).length
}
