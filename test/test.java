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

        System.out.println("Java runtime test:");
        System.out.println(toJson(payload));
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

    private static String toJson(Map<String, Object> map) {
        StringBuilder sb = new StringBuilder();
        sb.append("{\n");
        int index = 0;
        for (Map.Entry<String, Object> entry : map.entrySet()) {
            sb.append("  \"").append(entry.getKey()).append("\": ");
            Object value = entry.getValue();
            if (value instanceof Number || value instanceof Boolean) {
                sb.append(value);
            } else {
                sb
                    .append("\"")
                    .append(String.valueOf(value).replace("\"", "\\\""))
                    .append("\"");
            }
            if (index < map.size() - 1) {
                sb.append(",");
            }
            sb.append("\n");
            index++;
        }
        sb.append("}");
        return sb.toString();
    }
}
