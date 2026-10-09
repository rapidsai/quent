# SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
# SPDX-License-Identifier: Apache-2.0

import quent_tutorial_dag as quent


def main() -> None:
    with quent.Context() as context:
        plan = context.plan_observer().handle()
        plan.created()

        source = context.operator_observer().handle()
        source.declared(plan=plan)

        target = context.operator_observer().handle()
        target.declared(plan=plan)

        edge = context.plan_edge_observer().handle()
        edge.connected(plan=plan, source=source, target=target)


if __name__ == "__main__":
    main()
