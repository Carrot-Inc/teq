// An explicit `@scala.main` beside a program's own class named `main` makes the proxy, as dotty's
// `MainProxies` takes the resolved `scala.main` (the review's `QualifiedMain`).
class main extends scala.annotation.StaticAnnotation
@scala.main def qualified(n: Int): Unit = println(n)
