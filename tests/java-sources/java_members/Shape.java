public interface Shape {
  double area();
  static Shape unit() { return () -> 1.0; }
  default String describe() { return "area " + area(); }
}
