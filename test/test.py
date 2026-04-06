import datetime
import hashlib
import json
import math
import pathlib
import random
import sqlite3
import statistics
import textwrap
import uuid

import numpy as np
import pandas as pd
import requests


def main() -> None:
    numbers = [2, 4, 6, 8, 10]

    square_root = math.sqrt(81)
    mean_value = statistics.mean(numbers)

    random.seed(42)
    random_pick = random.choice(numbers)

    today = datetime.datetime.now().date().isoformat()
    current_dir = pathlib.Path.cwd()
    digest = hashlib.sha256("piebash".encode("utf-8")).hexdigest()[:16]
    sample_uuid = str(uuid.uuid5(uuid.NAMESPACE_DNS, "piebash.dev"))

    connection = sqlite3.connect(":memory:")
    cursor = connection.cursor()
    cursor.execute("CREATE TABLE scores (name TEXT, value INTEGER)")
    cursor.executemany(
        "INSERT INTO scores (name, value) VALUES (?, ?)",
        [("alpha", 10), ("beta", 20), ("gamma", 30)],
    )
    cursor.execute("SELECT COUNT(*), SUM(value) FROM scores")
    row_count, value_sum = cursor.fetchone()
    connection.close()

    wrapped = textwrap.fill(
        "PieBash should make runtime setup feel automatic and easy to understand.",
        width=42,
    )

    frame = pd.DataFrame({"name": ["alpha", "beta", "gamma"], "value": [10, 20, 30]})

    payload = {
        "today": today,
        "sqrt_81": square_root,
        "mean": mean_value,
        "random_pick": random_pick,
        "cwd": str(current_dir),
        "sha256_prefix": digest,
        "uuid5": sample_uuid,
        "sqlite_rows": row_count,
        "sqlite_sum": value_sum,
        "requests_version": requests.__version__,
        "numpy_sum": int(np.array(numbers).sum()),
        "pandas_mean": float(frame["value"].mean()),
    }

    print("Basic library functionality test:")
    print(json.dumps(payload, indent=2))
    print("\nWrapped text:")
    print(wrapped)


if __name__ == "__main__":
    main()
