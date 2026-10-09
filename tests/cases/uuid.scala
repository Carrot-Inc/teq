// java.util.UUID: fromString round trips and the JDK's validation, ordering, equality, hashes,
// the version and variant, and randomUUID checked for shape and uniqueness.
package uuidcase

import java.util.UUID

@main def main(): Unit =
  val texts = List(
    "123e4567-e89b-12d3-a456-426614174000",
    "00000000-0000-0000-0000-000000000000",
    "FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF",
    "f47ac10b-58cc-4372-a567-0e02b2c3d479",
    "7fffffff-ffff-7fff-bfff-ffffffffffff",
    "80000000-0000-4000-8000-000000000000",
    "1-2-3-4-5",
    "0001-02-0003-04-000005"
  )
  val ids = texts.map(UUID.fromString)
  for (t, u) <- texts.zip(ids) do
    println(s"$t -> $u msb=${u.getMostSignificantBits} lsb=${u.getLeastSignificantBits} v=${u.version} var=${u.variant} hash=${u.hashCode}")
  println(ids.sortWith(_.compareTo(_) < 0).mkString("\n"))
  println(ids.sortWith(_.compareTo(_) > 0).head)
  println(ids.head == UUID.fromString("123E4567-E89B-12D3-A456-426614174000"))
  println(ids.head.equals(ids(1)))
  println(s"${ids.head.compareTo(ids(1))} ${ids(1).compareTo(ids.head)} ${ids.head.compareTo(ids.head)}")
  println(ids(2).compareTo(ids(1)))
  println(new UUID(1L, 2L))
  println(new UUID(-1L, Long.MinValue))
  println(Map(ids.head -> "a", ids(1) -> "b")(UUID.fromString(texts.head)))
  println(Set(ids.head, UUID.fromString(texts.head)).size)
  for bad <- List("", "not-a-uuid", "123e4567-e89b-12d3-a456", "123e4567e89b12d3a456426614174000", "123e4567-e89b-12d3-a456-42661417400g", "123e4567-e89b-12d3-a456-4266141740001", "1-2-3-4", "12345678901234567-1-1-1-1", "x-1-2-3-4", "-1-2-3-4", "1--2-3-4") do
    try println(UUID.fromString(bad))
    catch case e: IllegalArgumentException => println(s"IllegalArgumentException: ${e.getMessage}")
  val r = List.fill(50)(UUID.randomUUID())
  println(r.distinct.size)
  println(r.forall(u => u.version == 4 && u.variant == 2))
  println(r.forall(u => u.toString.matches("[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}")))
  println(r.forall(u => UUID.fromString(u.toString) == u))

