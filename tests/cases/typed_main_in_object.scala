// A typed @main method of an object runs through the proxy dotty's MainProxies makes beside the object's package
// (mainMethods looks into objects, at any depth), which calls it through the object's path: run without
// arguments, its varargs are empty.
object Tool:
  object Inner:
    @main def count(xs: Int*): Unit = println(s"count ${xs.sum} of ${xs.size}")
