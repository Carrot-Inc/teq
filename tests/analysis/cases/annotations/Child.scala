// A program's class of the simple name of scalac's internal `@Child`, which the API keeps where
// it annotates a definition (Uses.scala): only `scala.annotation.internal`'s is left out.
package ann

import scala.annotation.Annotation

class Child extends Annotation
class Body extends Annotation
class Tag(c: Class[?]) extends Annotation
