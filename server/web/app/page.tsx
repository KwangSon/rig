"use client";

import { useState, useEffect } from "react";

const API_BASE = "http://localhost:3000/api/v1";

export default function Home() {
  const [projects, setProjects] = useState<string[]>([]);
  const [users, setUsers] = useState<any[]>([]);
  const [permissions, setPermissions] = useState<any[]>([]);
  const [isLoggedIn, setIsLoggedIn] = useState(false);
  const [token, setToken] = useState("");

  const [newProjectName, setNewProjectName] = useState("");
  const [newUserName, setNewUserName] = useState("");
  const [newUserEmail, setNewUserEmail] = useState("");

  const [permUserId, setPermUserId] = useState("");
  const [permProject, setPermProject] = useState("");
  const [permAccess, setPermAccess] = useState("read");

  useEffect(() => {
    const t = localStorage.getItem("token");
    if (t) {
      setToken(t);
      setIsLoggedIn(true);
    }
    fetchData();
  }, []);

  const fetchData = async () => {
    try {
      const [projRes] = await Promise.all([fetch(`${API_BASE}/projects`)]);
      setProjects(await projRes.json());

      if (isLoggedIn) {
        const [userRes, permRes] = await Promise.all([
          fetch(`${API_BASE}/users`, {
            headers: { Authorization: `Bearer ${token}` },
          }),
          fetch(`${API_BASE}/permissions`, {
            headers: { Authorization: `Bearer ${token}` },
          }),
        ]);
        setUsers(await userRes.json());
        setPermissions(await permRes.json());
      }
    } catch (e) {
      console.error("Failed to fetch data", e);
    }
  };

  const addProject = async (e: React.FormEvent) => {
    e.preventDefault();
    const res = await fetch(`${API_BASE}/create_project`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${token}`,
      },
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
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${token}`,
      },
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
    await fetch(`${API_BASE}/users/${id}`, {
      method: "DELETE",
      headers: { Authorization: `Bearer ${token}` },
    });
    fetchData();
  };

  const setPermission = async (e: React.FormEvent) => {
    e.preventDefault();
    await fetch(`${API_BASE}/permissions`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${token}`,
      },
      body: JSON.stringify({
        user_id: permUserId,
        project: permProject,
        access: permAccess,
      }),
    });
    fetchData();
  };

  return (
    <div className="min-h-screen bg-gray-50">
      <div className="mx-auto max-w-7xl py-6 sm:px-6 lg:px-8">
        <div className="px-4 py-6 sm:px-0">
          <h1 className="mb-8 text-3xl font-bold text-gray-900">Projects</h1>

          <div className="mb-8 grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3">
            {projects.map((p) => (
              <div
                key={p}
                className="overflow-hidden rounded-lg bg-white shadow"
              >
                <div className="p-6">
                  <h3 className="text-lg font-medium text-gray-900">{p}</h3>
                  <p className="mt-2 text-sm text-gray-500">
                    A project repository
                  </p>
                  <div className="mt-4">
                    <a
                      href={`/projects/${p}`}
                      className="text-sm font-medium text-indigo-600 hover:text-indigo-500"
                    >
                      View project →
                    </a>
                  </div>
                </div>
              </div>
            ))}
          </div>

          {isLoggedIn && (
            <div className="mb-8 rounded-lg bg-white p-6 shadow">
              <h2 className="mb-4 text-xl font-semibold text-gray-900">
                Create New Project
              </h2>
              <form onSubmit={addProject} className="flex gap-4">
                <input
                  type="text"
                  placeholder="Project Name"
                  value={newProjectName}
                  onChange={(e) => setNewProjectName(e.target.value)}
                  required
                  className="flex-1 rounded-md border border-gray-300 px-3 py-2 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 focus:outline-none"
                />
                <button
                  type="submit"
                  className="rounded-md bg-indigo-600 px-4 py-2 text-white hover:bg-indigo-700 focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:outline-none"
                >
                  Create
                </button>
              </form>
            </div>
          )}

          {isLoggedIn && (
            <>
              <div className="mb-8 rounded-lg bg-white p-6 shadow">
                <h2 className="mb-4 text-xl font-semibold text-gray-900">
                  Users
                </h2>
                <div className="overflow-x-auto">
                  <table className="min-w-full divide-y divide-gray-200">
                    <thead className="bg-gray-50">
                      <tr>
                        <th className="px-6 py-3 text-left text-xs font-medium tracking-wider text-gray-500 uppercase">
                          Name
                        </th>
                        <th className="px-6 py-3 text-left text-xs font-medium tracking-wider text-gray-500 uppercase">
                          Email
                        </th>
                        <th className="px-6 py-3 text-left text-xs font-medium tracking-wider text-gray-500 uppercase">
                          Actions
                        </th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-gray-200 bg-white">
                      {users.map((u) => (
                        <tr key={u.id}>
                          <td className="px-6 py-4 text-sm font-medium whitespace-nowrap text-gray-900">
                            {u.name}
                          </td>
                          <td className="px-6 py-4 text-sm whitespace-nowrap text-gray-500">
                            {u.email}
                          </td>
                          <td className="px-6 py-4 text-sm whitespace-nowrap text-gray-500">
                            <button
                              onClick={() => deleteUser(u.id)}
                              className="text-red-600 hover:text-red-900"
                            >
                              Delete
                            </button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>

                <h3 className="mt-6 mb-4 text-lg font-medium text-gray-900">
                  Add User
                </h3>
                <form onSubmit={addUser} className="flex gap-4">
                  <input
                    type="text"
                    placeholder="Name"
                    value={newUserName}
                    onChange={(e) => setNewUserName(e.target.value)}
                    required
                    className="flex-1 rounded-md border border-gray-300 px-3 py-2 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 focus:outline-none"
                  />
                  <input
                    type="email"
                    placeholder="Email"
                    value={newUserEmail}
                    onChange={(e) => setNewUserEmail(e.target.value)}
                    required
                    className="flex-1 rounded-md border border-gray-300 px-3 py-2 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 focus:outline-none"
                  />
                  <button
                    type="submit"
                    className="rounded-md bg-indigo-600 px-4 py-2 text-white hover:bg-indigo-700 focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:outline-none"
                  >
                    Add
                  </button>
                </form>
              </div>

              <div className="rounded-lg bg-white p-6 shadow">
                <h2 className="mb-4 text-xl font-semibold text-gray-900">
                  Permissions
                </h2>
                <div className="mb-6 overflow-x-auto">
                  <table className="min-w-full divide-y divide-gray-200">
                    <thead className="bg-gray-50">
                      <tr>
                        <th className="px-6 py-3 text-left text-xs font-medium tracking-wider text-gray-500 uppercase">
                          User
                        </th>
                        <th className="px-6 py-3 text-left text-xs font-medium tracking-wider text-gray-500 uppercase">
                          Project
                        </th>
                        <th className="px-6 py-3 text-left text-xs font-medium tracking-wider text-gray-500 uppercase">
                          Access
                        </th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-gray-200 bg-white">
                      {permissions.map((p, i) => {
                        const user = users.find((u) => u.id === p.user_id);
                        return (
                          <tr key={i}>
                            <td className="px-6 py-4 text-sm font-medium whitespace-nowrap text-gray-900">
                              {user ? user.name : p.user_id}
                            </td>
                            <td className="px-6 py-4 text-sm whitespace-nowrap text-gray-500">
                              {p.project}
                            </td>
                            <td className="px-6 py-4 text-sm whitespace-nowrap text-gray-500">
                              {p.access}
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>

                <h3 className="mb-4 text-lg font-medium text-gray-900">
                  Set Permission
                </h3>
                <form onSubmit={setPermission} className="flex gap-4">
                  <select
                    value={permUserId}
                    onChange={(e) => setPermUserId(e.target.value)}
                    required
                    className="rounded-md border border-gray-300 px-3 py-2 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 focus:outline-none"
                  >
                    <option value="">Select User</option>
                    {users.map((u) => (
                      <option key={u.id} value={u.id}>
                        {u.name}
                      </option>
                    ))}
                  </select>
                  <select
                    value={permProject}
                    onChange={(e) => setPermProject(e.target.value)}
                    required
                    className="rounded-md border border-gray-300 px-3 py-2 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 focus:outline-none"
                  >
                    <option value="">Select Project</option>
                    {projects.map((p) => (
                      <option key={p} value={p}>
                        {p}
                      </option>
                    ))}
                  </select>
                  <select
                    value={permAccess}
                    onChange={(e) => setPermAccess(e.target.value)}
                    className="rounded-md border border-gray-300 px-3 py-2 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 focus:outline-none"
                  >
                    <option value="read">Read</option>
                    <option value="write">Write</option>
                    <option value="admin">Admin</option>
                  </select>
                  <button
                    type="submit"
                    className="rounded-md bg-indigo-600 px-4 py-2 text-white hover:bg-indigo-700 focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:outline-none"
                  >
                    Set
                  </button>
                </form>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
