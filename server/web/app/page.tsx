"use client";

import { useState, useEffect } from "react";

const API_BASE = "http://localhost:3000";

export default function Home() {
  const [projects, setProjects] = useState<string[]>([]);
  const [users, setUsers] = useState<any[]>([]);
  const [permissions, setPermissions] = useState<any[]>([]);

  const [newProjectName, setNewProjectName] = useState("");
  const [newUserName, setNewUserName] = useState("");
  const [newUserEmail, setNewUserEmail] = useState("");

  const [permUserId, setPermUserId] = useState("");
  const [permProject, setPermProject] = useState("");
  const [permAccess, setPermAccess] = useState("read");

  useEffect(() => {
    fetchData();
  }, []);

  const fetchData = async () => {
    try {
      const [projRes, userRes, permRes] = await Promise.all([
        fetch(`${API_BASE}/projects`),
        fetch(`${API_BASE}/users`),
        fetch(`${API_BASE}/permissions`),
      ]);
      setProjects(await projRes.json());
      setUsers(await userRes.json());
      setPermissions(await permRes.json());
    } catch (e) {
      console.error("Failed to fetch data", e);
    }
  };

  const addProject = async (e: React.FormEvent) => {
    e.preventDefault();
    const res = await fetch(`${API_BASE}/create_project`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name: newProjectName }),
    });
    if (res.ok) {
      setNewProjectName("");
      fetchData();
    } else {
      const data = await res.json();
      alert(`Failed to create project: ${data.message || res.statusText}`);
    }
  };

  const addUser = async (e: React.FormEvent) => {
    e.preventDefault();
    await fetch(`${API_BASE}/users`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        name: newUserName,
        email: newUserEmail,
        role: "user",
      }),
    });
    setNewUserName("");
    setNewUserEmail("");
    fetchData();
  };

  const deleteUser = async (id: string) => {
    await fetch(`${API_BASE}/users/${id}`, { method: "DELETE" });
    fetchData();
  };

  const setPermission = async (e: React.FormEvent) => {
    e.preventDefault();
    await fetch(`${API_BASE}/permissions`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        user_id: permUserId,
        project: permProject,
        access: permAccess,
      }),
    });
    fetchData();
  };

  return (
    <div style={{ padding: "20px", fontFamily: "monospace" }}>
      <h1>Rig Admin</h1>
      <hr />

      <section>
        <h2>Projects</h2>
        <ul>
          {projects.map((p) => (
            <li key={p}>{p}</li>
          ))}
        </ul>

        <p>
          <strong>Create Project</strong>
        </p>
        <form onSubmit={addProject}>
          <input
            placeholder="Project Name"
            value={newProjectName}
            onChange={(e) => setNewProjectName(e.target.value)}
            required
          />{" "}
          <button type="submit">Create</button>
        </form>
      </section>
      <hr />

      <section>
        <h2>Users</h2>
        <table
          border={1}
          cellPadding={5}
          style={{ borderCollapse: "collapse", width: "100%", textAlign: "left" }}
        >
          <thead>
            <tr>
              <th>Name</th>
              <th>Email</th>
              <th>ID</th>
              <th>Action</th>
            </tr>
          </thead>
          <tbody>
            {users.map((u) => (
              <tr key={u.id}>
                <td>{u.name}</td>
                <td>{u.email}</td>
                <td>
                  <small>{u.id}</small>
                </td>
                <td>
                  <button onClick={() => deleteUser(u.id)}>Delete</button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>

        <p>
          <strong>Add User</strong>
        </p>
        <form onSubmit={addUser}>
          <input
            placeholder="Name"
            value={newUserName}
            onChange={(e) => setNewUserName(e.target.value)}
            required
          />{" "}
          <input
            placeholder="Email"
            value={newUserEmail}
            onChange={(e) => setNewUserEmail(e.target.value)}
            required
          />{" "}
          <button type="submit">Add</button>
        </form>
      </section>
      <hr />

      <section>
        <h2>Permissions</h2>
        <table
          border={1}
          cellPadding={5}
          style={{ borderCollapse: "collapse", width: "100%", textAlign: "left" }}
        >
          <thead>
            <tr>
              <th>User</th>
              <th>Project</th>
              <th>Access</th>
            </tr>
          </thead>
          <tbody>
            {permissions.map((p, i) => {
              const user = users.find((u) => u.id === p.user_id);
              return (
                <tr key={i}>
                  <td>{user ? user.name : p.user_id}</td>
                  <td>{p.project}</td>
                  <td>{p.access}</td>
                </tr>
              );
            })}
          </tbody>
        </table>

        <p>
          <strong>Set Permission</strong>
        </p>
        <form onSubmit={setPermission}>
          <select
            value={permUserId}
            onChange={(e) => setPermUserId(e.target.value)}
            required
          >
            <option value="">Select User</option>
            {users.map((u) => (
              <option key={u.id} value={u.id}>
                {u.name}
              </option>
            ))}
          </select>{" "}
          <select
            value={permProject}
            onChange={(e) => setPermProject(e.target.value)}
            required
          >
            <option value="">Select Project</option>
            {projects.map((p) => (
              <option key={p} value={p}>
                {p}
              </option>
            ))}
          </select>{" "}
          <select
            value={permAccess}
            onChange={(e) => setPermAccess(e.target.value)}
          >
            <option value="read">Read</option>
            <option value="write">Write</option>
            <option value="admin">Admin</option>
          </select>{" "}
          <button type="submit">Set</button>
        </form>
      </section>
    </div>
  );
}
