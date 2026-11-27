import a2.*
object Run { def main(args: Array[String]): Unit = { println(new MK().n); println(MO.n); println(new GK().n); println(GO.n); println(new PK().n); println(PO.n); class S extends MK; println(new S().n); class G extends GK; println(new G().n); println((new PK {}).n); println(Api.mono) } }
