// The collections a typical program touches: List, Vector, Map, Set, Option, for comprehensions.
case class Order(customer: String, item: String, qty: Int, price: Double)

def orders: List[Order] = List(
  Order("ann", "tea", 2, 3.5),
  Order("bob", "mug", 1, 8.0),
  Order("ann", "mug", 3, 8.0),
  Order("cy", "tea", 5, 3.5),
  Order("bob", "jar", 2, 4.25)
)

@main def run(): Unit =
  val byCustomer = orders.groupBy(_.customer)
  val totals = byCustomer.map((c, os) => (c, os.map(o => o.qty * o.price).sum))
  for (c, t) <- totals.toList.sortBy(_._1) do println(s"$c: $t")
  val items = orders.map(_.item).toSet
  println(items.toList.sorted.mkString(", "))
  val big = for
    o <- orders
    if o.qty > 1
    total = o.qty * o.price
    if total > 10
  yield s"${o.customer}/${o.item}"
  println(big.mkString("[", ", ", "]"))
  val v = Vector(3, 1, 2).sorted.map(_ * 2)
  println(v.foldLeft(0)(_ + _))
  println(orders.find(_.item == "jar").map(_.customer).getOrElse("none"))
  println(orders.zipWithIndex.collect { case (o, i) if i % 2 == 0 => o.item }.distinct)
  println(totals.get("dee").isEmpty)
