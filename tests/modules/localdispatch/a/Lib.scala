package lda

// Local traits whose members read the block's values, lifted as dotty's `LambdaLift` lifts them
// after `Mixin`: a call through the trait reaches its lifted method past a class's override of
// the source signature, a lazy val's initialiser runs at each read, a class nested in the trait
// at any depth holds what it reads, and a subtrait's `super` call goes through the class's
// accessor.
object Lib:
  def dispatch(n: Int): String =
    trait T:
      def get = n
      lazy val once = { n * 2 }
    class C extends T:
      override def get = 99
    val t: T = new C
    s"${t.get} ${t.once + t.once} ${(new C).get}"

  def deep(n: Int): Int =
    trait T:
      class A:
        class B:
          def get = n
        def get = (new B).get
      def get = (new A).get
    (new T {}).get

  def supertrait(n: Int): Int =
    trait T:
      def get: Int = n
    trait U extends T:
      override def get: Int = super.get + 1
    class C extends U
    (new C).get
