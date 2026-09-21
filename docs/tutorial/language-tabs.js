// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

(() => {
  "use strict";

  const languages = [
    { id: "rust", label: "Rust" },
    { id: "cpp", label: "C++" },
    { id: "python", label: "Python" },
  ];
  const languageIds = new Set(languages.map(({ id }) => id));
  const storageKey = "quent-tutorial-language";
  const examples = [];

  const loadLanguage = () => {
    try {
      const savedLanguage = localStorage.getItem(storageKey);
      return languageIds.has(savedLanguage) ? savedLanguage : "rust";
    } catch {
      return "rust";
    }
  };

  const saveLanguage = (language) => {
    try {
      localStorage.setItem(storageKey, language);
    } catch {
      // The tabs remain usable when browser storage is unavailable.
    }
  };

  const selectLanguage = (language, focus = false) => {
    examples.forEach(({ buttons, panels }) => {
      languages.forEach(({ id }) => {
        const selected = id === language;
        buttons[id].setAttribute("aria-selected", String(selected));
        buttons[id].tabIndex = selected ? 0 : -1;
        panels[id].hidden = !selected;
      });

      if (focus) {
        buttons[language].focus();
      }
    });

    saveLanguage(language);
  };

  const findLanguageExamples = (heading) => {
    const languageExamples = {};
    let element = heading.nextElementSibling;
    while (element && element.tagName !== "H2") {
      if (element.tagName === "PRE") {
        languages.forEach(({ id }) => {
          const code = element.querySelector(`code.language-${id}`);
          if (!languageExamples[id] && code) {
            languageExamples[id] = element;
          }
        });
      }
      element = element.nextElementSibling;
    }
    return languageExamples;
  };

  document.querySelectorAll("h2#instrumentation-api").forEach((heading, index) => {
    const languageExamples = findLanguageExamples(heading);
    if (!languageExamples.rust) {
      return;
    }

    const container = document.createElement("div");
    container.className = "language-example";
    languageExamples.rust.parentNode.insertBefore(
      container,
      languageExamples.rust,
    );

    const tabList = document.createElement("div");
    tabList.className = "language-tabs";
    tabList.setAttribute("role", "tablist");
    tabList.setAttribute("aria-label", "Instrumentation language");
    container.append(tabList);

    const buttons = {};
    const panels = {};

    languages.forEach(({ id, label }) => {
      const tabId = `language-tab-${index}-${id}`;
      const panelId = `language-panel-${index}-${id}`;
      const button = document.createElement("button");
      button.className = "language-tab";
      button.type = "button";
      button.id = tabId;
      button.textContent = label;
      button.setAttribute("role", "tab");
      button.setAttribute("aria-controls", panelId);
      button.addEventListener("click", () => selectLanguage(id));
      button.addEventListener("keydown", (event) => {
        const currentIndex = languages.findIndex((language) => language.id === id);
        let nextIndex;

        if (event.key === "ArrowLeft") {
          nextIndex = (currentIndex - 1 + languages.length) % languages.length;
        } else if (event.key === "ArrowRight") {
          nextIndex = (currentIndex + 1) % languages.length;
        } else if (event.key === "Home") {
          nextIndex = 0;
        } else if (event.key === "End") {
          nextIndex = languages.length - 1;
        } else {
          return;
        }

        event.preventDefault();
        selectLanguage(languages[nextIndex].id, true);
      });
      tabList.append(button);
      buttons[id] = button;

      const panel = document.createElement("div");
      panel.className = "language-panel";
      panel.id = panelId;
      panel.setAttribute("role", "tabpanel");
      panel.setAttribute("aria-labelledby", tabId);

      if (languageExamples[id]) {
        panel.append(languageExamples[id]);
      } else {
        const placeholder = document.createElement("p");
        placeholder.className = "language-placeholder";
        placeholder.textContent = `No ${label} wrapper example is available for this lesson.`;
        panel.append(placeholder);
      }

      container.append(panel);
      panels[id] = panel;
    });

    examples.push({ buttons, panels });
  });

  if (examples.length > 0) {
    selectLanguage(loadLanguage());
  }
})();
