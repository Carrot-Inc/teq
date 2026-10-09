// expect: 16:26: error: No ClassTag available for T
// expect: 17:25: error: No ClassTag available for B
// expect: 18:41: error: No ClassTag available for T
// expect: 19:12: error: No ClassTag available for T
// expect: 20:18: error: No ClassTag available for T
// expect: 21:20: error: No ClassTag available for T
// expect: 22:47: error: No ClassTag available for Array[T]
// expect: 23:56: error: No ClassTag available for T
// expect: 24:45: error: No ClassTag available for T
// expect: 25:13: error: No ClassTag available for Null
// expect: 26:15: error: No ClassTag available for Nothing
// expect: 11 errors found
// An array of an element type that is a type parameter needs the parameter's `ClassTag`, on
// every target: scalac's E172 with its wording and the type it names (`B` of `xs.toArray`,
// which nothing but its bounds determines), at the call where scalac points behind it.
def f[T](n: Int, x: T) = Array.fill[T](n)(x)
def g[T](xs: List[T]) = xs.toArray
def k[T](xs: Array[Int], f: Int => T) = xs.map(f)
def e[T] = Array.empty[T]
def a[T](x: T) = Array(x)
def m[T](n: Int) = new Array[T](n)
def nested[T](x: Array[T]): Array[Array[T]] = Array(x)
def mapped[T](a: Array[Int]): (Int => T) => Array[T] = a.map[T]
def filled[T](n: Int): (=> T) => Array[T] = Array.fill[T](n)
def nulls = Array[Null](null)
def nothing = Array.empty[Nothing]
@main def run(): Unit = println(f[String](1, "x").length)
