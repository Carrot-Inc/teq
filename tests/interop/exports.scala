// A library module: no @main, so loading it initialises the exported vals and, as the file object
// of scalac would, the other top-level vals of their file.
package shop

case class Product(name: String, price: Int)

@jsExport("version")
val version: String = "1.2"

@jsExport("catalog")
val catalog: List[Product] =
  println("catalog initialised")
  List(Product("pen", 3), Product("ink", 12))

val neverUsed: Int =
  println("not exported, initialised with its file")
  1

@jsExport("settings")
val settings: Any = js.obj("currency" -> "EUR", "rounding" -> 2)

@jsExport("greet")
def greet(name: String): String = s"hello, $name"

@jsExport("total")
def total(prices: Int*): Int = prices.toList.sum

@jsExport("label")
def label(prefix: String, parts: String*): String = prefix + parts.toList.mkString("[", "|", "]")

@jsExport("product")
def product(name: String, price: Int = 1): Product = Product(name, price)

@jsExport("describe")
def describe(p: Product): String = s"${p.name} costs ${p.price}"

@jsExport("cheaperThan")
def cheaperThan(limit: Int): List[Product] = catalog.filter(_.price < limit)

@jsExport("applyTwice")
def applyTwice(f: Int => Int)(x: Int): Int = f(f(x))

@jsExport("log")
def log(message: String): Unit = println(s"[shop] $message")

@jsExport("fromJs")
def fromJs(names: Any): List[String] = js.toList[String](names).map(_.toUpperCase)

// an import can be handed on as it is
@jsImport("node:path", "basename")
@jsExport("basename")
def basename(path: String): String

@jsExport("default")
val api: Any = js.obj("greet" -> ((name: String) => greet(name)), "version" -> version)
