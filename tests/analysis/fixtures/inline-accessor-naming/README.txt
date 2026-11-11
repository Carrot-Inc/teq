A.inc is scalac 3.8.4's -Ydump-sbt-inc -Yforce-sbt-phases output for A.scala:
  scala-cli --power compile -S 3.8.4 --jvm system --server=false -q \
    -O -Ydump-sbt-inc -O -Yforce-sbt-phases A.scala

User$anonClass is named and extensible, despite containing "$anon". Its getter
and setter use expanded accessor names. Factory's local class has an owner
path through a method, represented by "_$" in fullNameSeparated. The named class's used names and both classes' pickled accessor names must
match the recorded oracle after decoding NameTransformer escapes. The local
class's unused inline body is checked in TASTy, independently of dependency
collection.
