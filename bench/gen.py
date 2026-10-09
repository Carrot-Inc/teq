#!/usr/bin/env python3
"""Generates a synthetic Scala code base: gen.py <out dir> <files> <modules per file> [--packages] [--writable].

With --packages every file is a package of its own (bench.modN), which the split output lays out
as one module per file, as an application with many packages is. With --writable the module's
shapes that the TASTy writer withholds (`List.range`, the triple, a `for`
guard's `withFilter`) are written as shapes it states, so that every body of the products is
written."""
import os
import sys

args = [a for a in sys.argv[1:] if not a.startswith("--")]
out, n_files, per_file = args[0], int(args[1]), int(args[2])
packages = "--packages" in sys.argv
writable = "--writable" in sys.argv
os.makedirs(out, exist_ok=True)

MODULE = '''
enum Shape{i}:
  case Circle(r: Double)
  case Rect(w: Double, h: Double)
  case Empty

sealed trait Expr{i}
case class Num{i}(value: Int) extends Expr{i}
case class Add{i}(l: Expr{i}, r: Expr{i}) extends Expr{i}
case class Mul{i}(l: Expr{i}, r: Expr{i}) extends Expr{i}
case class Neg{i}(e: Expr{i}) extends Expr{i}

trait Show{i}[A]:
  def show(a: A): String

given Show{i}[Int] with
  def show(a: Int): String = "i" + a.toString

given listShow{i}[A](using s: Show{i}[A]): Show{i}[List[A]] with
  def show(xs: List[A]): String = xs.map(s.show).mkString("[", ", ", "]")

case class Account{i}(id: Int, owner: String, balance: Long, tags: List[String]):
  def deposit(amount: Long): Account{i} = copy(balance = balance + amount)
  def isRich: Boolean = balance > 1000000L
  def label: String = s"$owner#$id (${{tags.mkString(",")}})"

class Counter{i}(start: Int):
  private var count = start
  def inc(): Unit = count += 1
  def add(n: Int): Counter{i} =
    count += n
    this
  def get: Int = count

def eval{i}(e: Expr{i}): Int = e match
  case Num{i}(v) => v
  case Add{i}(l, r) => eval{i}(l) + eval{i}(r)
  case Mul{i}(l, r) => eval{i}(l) * eval{i}(r)
  case Neg{i}(inner) => -eval{i}(inner)

def simplify{i}(e: Expr{i}): Expr{i} = e match
  case Add{i}(Num{i}(0), r) => simplify{i}(r)
  case Add{i}(l, Num{i}(0)) => simplify{i}(l)
  case Mul{i}(Num{i}(1), r) => simplify{i}(r)
  case Mul{i}(l, Num{i}(1)) => simplify{i}(l)
  case Add{i}(l, r) => Add{i}(simplify{i}(l), simplify{i}(r))
  case Mul{i}(l, r) => Mul{i}(simplify{i}(l), simplify{i}(r))
  case other => other

def area{i}(s: Shape{i}): Double = s match
  case Shape{i}.Circle(r) => 3.14159 * r * r
  case Shape{i}.Rect(w, h) => w * h
  case Shape{i}.Empty => 0.0

def describe{i}(x: Int | String): String = x match
  case n: Int => s"int $n"
  case s: String => s"string ${{s.length}}"

def stats{i}(xs: List[Int]): (Int, Int, Double) =
  val total = xs.foldLeft(0)(_ + _)
  val biggest = xs.foldLeft(0)((a, b) => if a > b then a else b)
  val mean = if xs.isEmpty then 0.0 else total.toDouble / xs.length
  (total, biggest, mean)

def pairs{i}(n: Int): List[(Int, Int)] =
  for
    a <- List.range(0, n)
    b <- List.range(a, n)
    if (a + b) % 3 == 0
  yield (a, b)

def collatz{i}(start: Int): Int =
  var n = start
  var steps = 0
  while n != 1 do
    n = if n % 2 == 0 then n / 2 else 3 * n + 1
    steps += 1
  steps

def process{i}(accounts: List[Account{i}]): String =
  val rich = accounts.filter(_.isRich).map(_.label)
  val total = accounts.map(_.balance).sum
  val shown = summon[Show{i}[List[Int]]].show(accounts.map(_.id))
  s"${{rich.length}} rich of ${{accounts.length}}, total $total, ids $shown"

def run{i}(): Int =
  val e = Add{i}(Num{i}({i}), Mul{i}(Num{i}(2), Neg{i}(Num{i}(3))))
  val c = Counter{i}(eval{i}(simplify{i}(e)))
  c.inc()
  c.add(collatz{i}(27)).add(pairs{i}(6).length)
  val accounts = List(Account{i}(1, "ann", 5000000L, List("a", "b")), Account{i}(2, "bob", 10L, Nil))
  val text = process{i}(accounts.map(_.deposit(5L))) + describe{i}({i}) + describe{i}("x")
  val (total, biggest, mean) = stats{i}(List(1, 2, 3, {i}))
  c.get + text.length + total + biggest + mean.toInt + area{i}(Shape{i}.Rect(2.0, 3.0)).toInt
'''

# The writable variant's replacements of the module's text.
WRITABLE = [
    ('''def stats{i}(xs: List[Int]): (Int, Int, Double) =
  val total = xs.foldLeft(0)(_ + _)
  val biggest = xs.foldLeft(0)((a, b) => if a > b then a else b)
  val mean = if xs.isEmpty then 0.0 else total.toDouble / xs.length
  (total, biggest, mean)''',
     '''case class Stats{i}(total: Int, biggest: Int, mean: Double)

def stats{i}(xs: List[Int]): Stats{i} =
  val total = xs.foldLeft(0)(_ + _)
  val biggest = xs.foldLeft(0)((a, b) => if a > b then a else b)
  val mean = if xs.isEmpty then 0.0 else total.toDouble / xs.length
  Stats{i}(total, biggest, mean)'''),
    ('''def pairs{i}(n: Int): List[(Int, Int)] =
  for
    a <- List.range(0, n)
    b <- List.range(a, n)
    if (a + b) % 3 == 0
  yield (a, b)''',
     '''def upto{i}(from: Int, until: Int): List[Int] =
  var out: List[Int] = Nil
  var k = until - 1
  while k >= from do
    out = k :: out
    k -= 1
  out

def pairs{i}(n: Int): List[(Int, Int)] =
  for
    a <- upto{i}(0, n)
    b <- upto{i}(a, n).filter(b => (a + b) % 3 == 0)
  yield (a, b)'''),
    ("  val (total, biggest, mean) = stats{i}(List(1, 2, 3, {i}))", "  val Stats{i}(total, biggest, mean) = stats{i}(List(1, 2, 3, {i}))"),
]
if writable:
    for old, new in WRITABLE:
        assert old in MODULE, old
        MODULE = MODULE.replace(old, new)

count = 0
for f in range(n_files):
    with open(os.path.join(out, f"mod{f}.scala"), "w") as fh:
        fh.write(f"package bench.mod{f}\n" if packages else "package bench\n")
        for m in range(per_file):
            fh.write(MODULE.replace("{i}", str(count)).replace("{{", "{").replace("}}", "}"))
            count += 1

with open(os.path.join(out, "main.scala"), "w") as fh:
    fh.write("package bench\n\n")
    if packages:
        fh.write("".join(f"import bench.mod{f}.*\n" for f in range(n_files)))
    fh.write("\n@main def benchMain(): Unit =\n  var total = 0L\n")
    for i in range(count):
        fh.write(f"  total += run{i}()\n")
    fh.write("  println(total)\n")
print(count, "modules")
