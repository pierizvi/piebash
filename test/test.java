import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import java.nio.file.Paths;
import java.security.MessageDigest;
import java.time.LocalDate;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;

class TestApp {

    public static void main(String[] args) throws Exception {
        List<Integer> numbers = List.of(2, 4, 6, 8, 10);
        Gson gson = new GsonBuilder().setPrettyPrinting().create();

        double sqrt81 = Math.sqrt(81);
        double mean = numbers
            .stream()
            .mapToInt(Integer::intValue)
            .average()
            .orElse(0.0);
        int randomPick = numbers.get(new Random(42).nextInt(numbers.size()));

        String today = LocalDate.now().toString();
        String cwd = Paths.get("").toAbsolutePath().normalize().toString();
        String shaPrefix = sha256Hex("piebash").substring(0, 16);
        String javaVersion = System.getProperty("java.version");

        Map<String, Object> payload = new LinkedHashMap<>();
        payload.put("today", today);
        payload.put("sqrt_81", sqrt81);
        payload.put("mean", mean);
        payload.put("random_pick", randomPick);
        payload.put("cwd", cwd);
        payload.put("java_version", javaVersion);
        payload.put("sha256_prefix", shaPrefix);
        payload.put("gson_version", packageVersion(Gson.class));

        System.out.println("Java runtime + dependency test:");
        System.out.println(gson.toJson(payload));
    }

    private static String sha256Hex(String value) throws Exception {
        MessageDigest digest = MessageDigest.getInstance("SHA-256");
        byte[] hash = digest.digest(value.getBytes("UTF-8"));
        StringBuilder out = new StringBuilder();
        for (byte b : hash) {
            out.append(String.format("%02x", b));
        }
        return out.toString();
    }

    private static String packageVersion(Class<?> type) {
        Package pkg = type.getPackage();
        String version = pkg != null ? pkg.getImplementationVersion() : null;
        return version != null ? version : "unknown";
    }
}
