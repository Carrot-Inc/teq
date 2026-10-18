//> using file ../deps.scala
//> using file ../tools/src
// deps.scala asks cats-core at 2.12.0, which the build's resolution evicted: the lock records 2.13.0 in its place.
println(cats.data.NonEmptyList.of(1, 2, 3).toList.sum)
println(envfix.Entry("probe", 1).name)
