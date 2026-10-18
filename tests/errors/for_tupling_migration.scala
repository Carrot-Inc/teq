// teq: --source 3.8 --wconf msg=multiple.val:e
// `DropForMap`'s `TuplingMigrationForMap` (the review of 1888ac4df, dotty's `tests/neg/i25077.scala`): the better
// path's application over more than one alias, whose receiver's `map` alternatives differ in their final result
// types, is warned of, made an error by `-Wconf`.
def f(x: Int): (Int, Int) = (1, x)
val result: Iterable[(Int, Int)] =
  for
    (k, v) <- Map(1 -> 1, 2 -> 1, 3 -> 1)
    x      = k + v
    (a, b) = f(x)
    (y, z) <- Map(42 -> 27)
  yield (a+y, b+z)
val l =
  for
    k <- List(1, 2)
    x = k + 1
    y = x + 1
    z <- List(10)
  yield y + z
// expect: 7:3: error: For comprehension with multiple val assignments may change result type in 3.8
// absent: for_tupling_migration.scala:14:
