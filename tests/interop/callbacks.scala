// teq function values are JS functions: JS APIs call them back directly.

case class Item(name: String, price: Int)

def show(x: Any): String = js.cast[String](js.call(js.global("JSON"), "stringify", x))

def halve(x: Int): Int = x / 2

@main def main(): Unit =
  val numbers = js.array(3, 1, 2)
  println(show(js.call(numbers, "map", (x: Int) => x * 2)))
  println(show(js.call(numbers, "map", (x: Int, i: Int) => s"$i:$x")))
  println(show(js.call(numbers, "filter", (x: Int) => x > 1)))
  println(js.call(numbers, "reduce", (acc: Int, x: Int) => acc + x, 0))
  js.call(numbers, "sort", (a: Int, b: Int) => a - b)
  println(show(numbers))

  js.call(numbers, "forEach", (x: Int) => println(s"item $x"))

  var calls = 0
  js.call(numbers, "forEach", (_: Int) => calls += 1)
  println(calls)

  def square(x: Int): Int = x * x
  val squareFn: Int => Int = square
  println(show(js.call(numbers, "map", squareFn)))

  // a def named without arguments is the same JS function every time, which is what lets a
  // library such as React recognise a component
  println(show(js.call(js.array(10, 20), "map", halve)))
  val same = js.construct(js.global("Function"), "f", "g", "return f === g;")
  println(js.apply(same, halve, halve))

  // teq values travel through JS untouched
  val items = js.array(Item("pen", 3), Item("ink", 12), Item("pad", 7))
  val cheap = js.call(items, "filter", (i: Item) => i.price < 10)
  println(js.toList[Item](cheap))
  val found = js.call(items, "find", (i: Item) => i.name == "ink")
  println(js.cast[Item](found).price)
  println(js.isUndefined(js.call(items, "find", (i: Item) => i.price > 100)))

  // a curried function is a JS function returning a JS function
  val add: Int => Int => Int = a => b => a + b
  println(js.apply(js.apply(add, 1), 2))

  // a JS function handed back to teq is called like any other function value
  val parse = js.cast[String => Any](js.get(js.global("JSON"), "parse"))
  println(js.get(parse("{\"ok\": true}"), "ok"))
  val compose = js.construct(js.global("Function"), "f", "g", "return (x) => g(f(x));")
  val inc = (x: Int) => x + 1
  val incThenDouble = js.cast[Int => Int](js.apply(compose, inc, (x: Int) => x * 2))
  println(List(1, 2, 3).map(incThenDouble))

  // callbacks that fire after main has returned still print, in order
  js.apply(js.global("setTimeout"), () => println("timer fired"), 0)
  val promise = js.call(js.global("Promise"), "resolve", 7)
  js.call(promise, "then", (v: Int) => println(s"promise resolved with $v"))
  js.apply(js.global("queueMicrotask"), () => print("micro"))
  js.apply(js.global("queueMicrotask"), () => println("task"))
  println("main done")
