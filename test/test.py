import datetime
import json
import math
import pathlib
import random
import statistics


def main() -> None:
    numbers = [2, 4, 6, 8, 10]

    # math + statistics
    square_root = math.sqrt(81)
    mean_value = statistics.mean(numbers)

    # random
    random.seed(42)
    random_pick = random.choice(numbers)

    # datetime
    today = datetime.datetime.now().date().isoformat()

    # pathlib
    current_dir = pathlib.Path.cwd()

    # json
    payload = {
        "today": today,
        "sqrt_81": square_root,
        "mean": mean_value,
        "random_pick": random_pick,
        "cwd": str(current_dir),
    }

    print("Basic library functionality test:")
    print(json.dumps(payload, indent=2))


if __name__ == "__main__":
    main()
