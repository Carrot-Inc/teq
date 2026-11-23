// ZipWriter <out> <root> <mode>: the files under <root> written into the jar <out> by the JDK's own writers, as a
// library's build writes them with java.util.zip: `stored` (ZipOutputStream, STORED, each entry's size and CRC-32 set
// before it), `deflated` (DEFLATED, sizes in data descriptors), `deflated-1` and `deflated-9` (the levels), `commented`
// (a comment on the archive and on each entry), `directories` (an entry for each directory), `jar` (JarOutputStream
// with a manifest). Run as a single source file: `java tests/native-build/ZipWriter.java <out> <root> <mode>`.
import java.io.*;
import java.nio.file.*;
import java.util.*;
import java.util.jar.*;
import java.util.zip.*;

public class ZipWriter {
    public static void main(String[] args) throws IOException {
        Path out = Paths.get(args[0]), root = Paths.get(args[1]);
        String mode = args[2];
        List<Path> paths = new ArrayList<>();
        try (var s = Files.walk(root)) { s.filter(p -> !p.equals(root)).sorted().forEach(paths::add); }
        try (OutputStream file = Files.newOutputStream(out)) {
            ZipOutputStream zip = mode.equals("jar") ? new JarOutputStream(file, manifest()) : new ZipOutputStream(file);
            if (mode.startsWith("deflated-")) zip.setLevel(Integer.parseInt(mode.substring(9)));
            if (mode.equals("commented")) zip.setComment("a comment on the archive");
            for (Path p : paths) {
                String name = root.relativize(p).toString().replace(File.separatorChar, '/');
                if (Files.isDirectory(p)) {
                    if (mode.equals("directories") || mode.equals("jar")) { zip.putNextEntry(new ZipEntry(name + "/")); zip.closeEntry(); }
                    continue;
                }
                byte[] data = Files.readAllBytes(p);
                ZipEntry e = new ZipEntry(name);
                if (mode.equals("stored")) {
                    CRC32 crc = new CRC32();
                    crc.update(data);
                    e.setMethod(ZipEntry.STORED);
                    e.setSize(data.length);
                    e.setCompressedSize(data.length);
                    e.setCrc(crc.getValue());
                }
                if (mode.equals("commented")) e.setComment("a comment on " + name);
                zip.putNextEntry(e);
                zip.write(data);
                zip.closeEntry();
            }
            zip.finish();
        }
    }

    static Manifest manifest() {
        Manifest m = new Manifest();
        m.getMainAttributes().put(Attributes.Name.MANIFEST_VERSION, "1.0");
        m.getMainAttributes().put(new Attributes.Name("Created-By"), "ZipWriter");
        return m;
    }
}
