// expect: 9:45: error: No ClassTag available for T
// expect: 10:35: error: No ClassTag available for B
// expect: 11:55: error: No ClassTag available for Array[T]
// expect: 12:47: error: No ClassTag available for T
// expect: 4 errors found
// teq: --target jvm
// On the JVM, where the evidence is a parameter, a tag that is not found is reported in
// scalac's words too, as where the evidence is erased (class_tag_missing.scala).
def f[T](n: Int, x: T) = Array.fill[T](n)(x)
def g[T](xs: List[T]) = xs.toArray
def nested[T](x: Array[T]): Array[Array[T]] = Array(x)
def tag[T] = summon[scala.reflect.ClassTag[T]]
@main def run(): Unit = println(f[String](1, "x").length)
