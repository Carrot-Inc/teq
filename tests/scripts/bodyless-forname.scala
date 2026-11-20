// Class.forName initializes the class it finds, and its body-less Java superclass's static initializer with it
// (JLS 12.4.1): refused at the call. classOf and a lookup that does not initialize run.
class Shelf extends jlib.Registry

@main def run(): Unit =
  println(classOf[Shelf].getName)
  println(Class.forName("Shelf", false, classOf[Shelf].getClassLoader).getName)
  println(Class.forName("Shelf").getName)
