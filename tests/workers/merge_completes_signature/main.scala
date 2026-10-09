// Each object's anonymous class merges its inherited `put` when the body making it is published
// (`settle_registered_merges`), and the traits after the object have their signatures completed
// by that merge: their publication inside the merge registers the class only once its merge is
// settled. scalac prints the sixteen lines below.
@main def run(): Unit =
  val made = List(U0.make(), U1.make(), U2.make(), U3.make(), U4.make(), U5.make(), U6.make(), U7.make(),
    U8.make(), U9.make(), U10.make(), U11.make(), U12.make(), U13.make(), U14.make(), U15.make())
  for (q, i) <- made.zipWithIndex do
    q.put(i - 1)
    q.put(i)
    q.put(i + 1)
    println(q.items.mkString(","))
