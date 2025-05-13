# 🚂 The Train Journey – A Soulful Interactive Portfolio

A soulful interactive portfolio — part game, part story, all vibes. Built from scratch with Rust, WASM, and a sprinkle of magic. Switch between raw personal journeys and recruiter-ready milestones. No frameworks. Just code, art, and chaos stitched together like a dream on rails.

## 🎯 Vision
Create a deeply personal, technically impressive, and emotionally resonant portfolio that feels like a journey. It's not just a website — it's a **living storybook**, driven by **personality**, **design**, and **custom code**.

---

## 🎨 Theme
- A tiny 2D character arrives at a train station.
- A train moves **left to right**, stopping at milestones like:
  - Education
  - Work
  - Skills
  - Projects
- The background is a **real-time, mood-reactive gradient** — sunset-style, powered by procedural noise.

---

## 🌓 Modes

### 🔵 Professional Mode
- Clean & recruiter-friendly.
- Direct, minimal emotion.
- No-nonsense storytelling at each stop.

### 🟣 Personal Mode
- Raw, real, emotional.
- Narrator tells the story behind each milestone.
- Voiceover (self-recorded or AI-generated).
- Subtitles for accessibility.

---

## 🛤️ Narrative Structure

- Character drops in from top of screen.
- Breaks the 4th wall:
  > “Oh, hi! Didn't see you there. Wondering how I ended up here? Let’s go on a train ride.”
- User prompted: **Press E to begin**
- Train stops at key milestones.
- Character gets off, tells the story.
- Background reacts to emotional tone.

---

## 🖌️ Design Direction

- 2D side-scroller layout
  - Bottom third: train & station
  - Middle: text & interaction
  - Backdrop: dynamic mood gradient
- Hand-drawn, soulful art (custom or by sister)
- Clean layout, fluid transitions
- Creative info expansion per milestone (no popups — maybe unfolding compartments)
- Fully responsive & accessible

---

## 🧠 Technical Goals

### ⚙️ Core Stack

- **Rust + WebAssembly (WASM)**:  
  Logic, state, audio sync, procedural visuals

- **TypeScript + Vite**:  
  DOM control, UI interaction, frontend glue

- **HTML / CSS** (or CSS-in-Rust if needed):  
  Layout, styling, transitions

### 🔧 Optional Enhancements

- **WebGL / Shaders**: for advanced animated background effects
- **Voiceover System**:
  - Self-recorded voice acting OR
  - AI-generated via ElevenLabs
  - Synced with subtitles & background mood
- **CMS Integration**:
  - Start with local Markdown/JSON
  - Upgrade path: Netlify CMS, Sanity, or custom parser

---

## 🧱 Development Phases

### ✅ Phase 0 – Setup
- [x] Rust + WASM via `wasm-pack`
- [x] Vite + TypeScript frontend
- [x] Modular project folder structure

### 🚧 Phase 1 – Core Engine
- [ ] Character logic: idle / walk / interact
- [ ] Station detection system
- [ ] Mood-reactive background engine
- [ ] Render first milestone
- [ ] "Press E" prompt system

### 🔄 Phase 2 – Interaction Layer
- [ ] Dialog system + subtitles
- [ ] Voice playback sync
- [ ] Toggle between Professional & Personal modes
- [ ] Conditional content per mode

### 🚀 Phase 3 – Expansion
- [ ] Mood FX (sunset, fog, glow, lighting)
- [ ] Creative info panels
- [ ] CMS backend for dynamic content
- [ ] SEO, accessibility polish, deploy

### 🌍 Phase 4 – Showcase
- [ ] Launch to HackerNews / Reddit / Dev.to
- [ ] Behind-the-scenes devlog
- [ ] Postmortem reflections (on-site)

---

## ⚠️ UX & Risk Considerations

- 🎞️ **Too much animation?**  
  → Include skip, fast-forward, or collapse options

- 📱 **Responsiveness?**  
  → Train layout adapts for mobile/tablet

- 🚀 **Performance?**  
  → Move heavy lifting to WASM (backgrounds, audio, state)

- ♿ **Accessibility?**  
  → Subtitles for all voiceovers, keyboard nav

- 🧭 **Too abstract?**  
  → Use subtle prompts or hints

---

## 💎 Your Unique Edge

- Not a cookie-cutter React portfolio
- **Custom-built from scratch** in Rust
- Visually striking, full of personality
- Deeply narrative + emotionally honest
- A tech flex *and* a storybook

---

## ✅ Where To Go From Here

Start small. Prove one piece.

### Starter Checklist:
- [ ] Set up Rust/WASM environment
- [ ] Build test gradient background with procedural noise
- [ ] Display one test station with placeholder data
- [ ] Animate character drop-in & walk
- [ ] Create “Press E to begin” interaction
- [ ] Add mode toggle logic

---

> Let the train journey begin. 🚂✨

---

# Product Requirements Document (PRD)  
**Project:** The Train Journey – A Soulful Interactive Portfolio  
**Owner:** [Your Name]  
**Date:** [Today’s Date]

---

## 1. **Executive Summary**

Create a deeply personal, technically impressive, and emotionally resonant portfolio website. The experience is a metaphorical train journey, where a character travels through milestones (education, work, skills, projects), with a mood-reactive background and dual narrative modes (Professional/Personal). The stack leverages Rust + WASM for performance and technical credibility, with a TypeScript/Vite frontend.

---

## 2. **Goals & Success Criteria**

### **Primary Goals**
- Deliver a unique, narrative-driven portfolio that stands out to developers, designers, and recruiters.
- Demonstrate technical mastery (Rust, WASM, procedural graphics, accessibility).
- Allow users to explore both professional and personal stories.

### **Success Criteria**
- Site loads in <2s on desktop and mobile.
- All milestones and modes are accessible and navigable.
- Positive feedback from at least 10 developers/designers/recruiters.
- At least 1,000 unique visitors within 3 months of launch.

---

## 3. **User Personas**

- **Recruiter:** Wants a quick, clear view of skills and experience.
- **Developer:** Interested in technical implementation and code quality.
- **Designer:** Looks for unique visuals, UX, and storytelling.
- **General Visitor:** Curious about your journey and personality.

---

## 4. **Features & Requirements**

### **Core Features**
- **2D Side-Scroller Train Journey:**  
  - Character arrives at a station, boards a train, and stops at milestones.
  - Each milestone reveals content (text, images, voiceover).
- **Mood-Reactive Background:**  
  - Procedural gradient changes with story mood.
- **Dual Narrative Modes:**  
  - Professional: Factual, concise.
  - Personal: Honest, emotional, with voiceover and subtitles.
- **Responsive & Accessible:**  
  - Works on all devices, keyboard navigation, ARIA, subtitles.
- **Mode Toggle:**  
  - Switch between Professional and Personal at any time.

### **Optional/Stretch Features**
- **Voiceover System:**  
  - AI or recorded voice, synced with subtitles.
- **CMS Integration:**  
  - Markdown or headless CMS for milestone content.
- **Visual FX:**  
  - Lighting, fog, sunset, etc.
- **Devlog/Behind-the-Scenes Page:**  
  - Share process, code, and reflections.

---

## 5. **Technical Architecture**

- **Frontend:**  
  - Vite + TypeScript for UI, state, and DOM.
- **Core Logic & Animation:**  
  - Rust + WASM for performance, procedural backgrounds, and state management.
- **Assets:**  
  - Hand-drawn SVGs/PNGs for character, train, stations.
- **Voiceover:**  
  - AI-generated or recorded, with text sync.
- **CMS:**  
  - Markdown files or headless CMS (optional, for scalability).

---

## 6. **Milestones & Timeline**

| Phase            	| Tasks                                                             	| Timeline     	|
|----------------------|-----------------------------------------------------------------------|------------------|
| **Phase 0: Setup**   | Rust+WASM, Vite+TS, project structure, hello world               	| 2 days       	|
| **Phase 1: Engine**  | Background engine, static train/station, character drop/idle, prompt | 1 week       	|
| **Phase 2: Interact**| Character movement, milestone panels, mode toggle, subtitles     	| 2 weeks      	|
| **Phase 3: Polish**  | Responsive, accessibility, CMS, FX, more animation               	| 2-4 weeks    	|
| **Phase 4: Launch**  | SEO, deploy, analytics, devlog, share                            	| 1 week       	|

**Total Estimated Duration:** 5-7 weeks

---

## 7. **Risks & Mitigation**

- **Animation Overload:**  
  - Provide skip/collapse options.
- **Performance:**  
  - Offload heavy tasks to WASM, optimize assets.
- **Accessibility:**  
  - Subtitles, keyboard nav, ARIA labels.
- **User Confusion:**  
  - Subtle prompts, onboarding, help overlay.

---

## 8. **KPIs & Measurement**

- Time on site
- Completion rate of journey
- Mode toggle usage
- Feedback from target personas
- Site performance metrics

---

## 9. **Roadmap (Gantt-style)**

```plaintext
Week 1:  |██████████| Phase 0: Setup
Week 2:  |██████████| Phase 1: Engine
Week 3-4:|████████████████| Phase 2: Interact
Week 5-6:|████████████████| Phase 3: Polish
Week 7:  |██████████| Phase 4: Launch
```

---

## 10. **Appendix**

- **References:**  
  - [wasm-pack](https://rustwasm.github.io/wasm-pack/)
  - [Vite](https://vitejs.dev/)
  - [Rust + WASM Book](https://rustwasm.github.io/docs/book/)
- **Inspiration:**  
  - Gris (game), Spiritfarer, VS Code onboarding, interactive storybooks

---

**End of PRD**

---