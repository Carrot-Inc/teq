class Child extends Outer<String> {
  public Inner getInner() {
    return new Inner() {
      public void foo(String e) { System.out.println("got " + e); }
    };
  }
}
